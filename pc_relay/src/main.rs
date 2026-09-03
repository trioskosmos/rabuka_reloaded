// PC Relay Server for 3DS Crossplay
//
// Architecture:
//   3DS (UDP) ──► [PC Relay: UDP listener + GameState] ──► WebSocket clients (browser)
//                    │
//                    ▼
//            Deterministic engine
//            (same seed → same state)
//            Only ActionSync (~20 bytes) forwarded

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::{BufMut, BytesMut};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use rabuka_engine::{
    card::CardDatabase,
    deck_builder::DeckBuilder,
    deck_parser::{DeckList, DeckParser},
    game_setup, game_state::GameState, player::Player,
    turn::TurnEngine,
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{UdpSocket, TcpListener},
    sync::{mpsc, Mutex},
    time::interval,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info};
use uuid::Uuid;

const MAGIC: u32 = 0x52424B50; // "RBKP"
const PORT: u16 = 7341;
const MAX_PACKET: usize = 4096;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const SESSION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Parser, Debug)]
#[command(name = "pc_relay", about = "PC Relay Server for 3DS Crossplay")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:7341")]
    udp_addr: SocketAddr,
    #[arg(long, default_value = "0.0.0.0:7342")]
    ws_addr: SocketAddr,
    #[arg(long, default_value = "../cards/cards.json")]
    cards_path: String,
    #[arg(long, default_value = "../cards/decks/")]
    decks_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionSync {
    action_tag: u16,
    card_id: Option<i16>,
    card_indices: Vec<usize>,
    stage_area: u8,
    use_baton_touch: bool,
    ability_index: Option<u16>,
    action_seq: u32,
    player_id: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeckSync {
    seed: u64,
    p1_main_templates: Vec<u16>,
    p1_energy_templates: Vec<u16>,
    p2_main_templates: Vec<u16>,
    p2_energy_templates: Vec<u16>,
}

#[derive(Debug)]
enum ClientType {
    ThreeDS { addr: SocketAddr, player_id: u8 },
    WebSocket { id: Uuid, player_id: u8, tx: mpsc::UnboundedSender<Message> },
}

#[derive(Debug)]
struct Session {
    id: Uuid,
    game_state: Arc<Mutex<GameState>>,
    clients: HashMap<Uuid, ClientType>,
    three_ds_addr: Option<SocketAddr>,
    last_activity: Instant,
    is_host: bool,
    deck_sync: Option<DeckSync>,
}

impl Session {
    fn new(deck_sync: DeckSync, cards: Arc<Vec<rabuka_engine::card::Card>>, _decks: &[DeckList]) -> anyhow::Result<Self> {
        let mut cards_vec = (*cards).clone();
        rabuka_engine::card_loader::CardLoader::attach_abilities(&mut cards_vec);
        let mut db = Arc::new(CardDatabase::load_or_create(cards_vec));

        let build_from_templates = |db: &mut Arc<CardDatabase>, templates: &Vec<u16>| -> anyhow::Result<rabuka_engine::deck_builder::Deck> {
            let mut deck = rabuka_engine::deck_builder::Deck {
                main_deck: std::collections::VecDeque::new(),
                energy_deck: std::collections::VecDeque::new(),
            };
            for &tid in templates {
                let cid = Arc::make_mut(db).create_copy(tid as i16);
                if let Some(card) = db.get_card(cid) {
                    match card.card_type {
                        rabuka_engine::card::CardType::Energy => deck.energy_deck.push_back(cid),
                        _ => deck.main_deck.push_back(cid),
                    }
                }
            }
            Ok(deck)
        };

        let mut pd1 = build_from_templates(&mut db, &deck_sync.p1_main_templates)?;
        let mut pd2 = build_from_templates(&mut db, &deck_sync.p2_main_templates)?;
        DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
        DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
        rabuka_engine::rng::seed(deck_sync.seed as u32);
        pd1.shuffle_main_deck(); pd1.shuffle_energy_deck();
        pd2.shuffle_main_deck(); pd2.shuffle_energy_deck();

        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        p1.set_main_deck(pd1.main_deck); p1.set_energy_deck(pd1.energy_deck);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p2.set_main_deck(pd2.main_deck); p2.set_energy_deck(pd2.energy_deck);
        let mut gs = GameState::new(p1, p2, db);
        game_setup::setup_game(&mut gs);

        Ok(Self {
            id: Uuid::new_v4(),
            game_state: Arc::new(Mutex::new(gs)),
            clients: HashMap::new(),
            three_ds_addr: None,
            last_activity: Instant::now(),
            is_host: true,
            deck_sync: Some(deck_sync),
        })
    }

    fn add_client(&mut self, client: ClientType) -> Uuid {
        let id = match &client {
            ClientType::ThreeDS { .. } => Uuid::nil(),
            ClientType::WebSocket { id, .. } => *id,
        };
        self.clients.insert(id, client);
        self.last_activity = Instant::now();
        id
    }

    fn remove_client(&mut self, id: &Uuid) {
        self.clients.remove(id);
        self.last_activity = Instant::now();
    }

    fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > SESSION_TIMEOUT && self.clients.is_empty()
    }

    async fn broadcast_action(&self, action: &ActionSync, exclude: Option<&Uuid>) {
        let bytes = serde_json::to_vec(action).unwrap();
        let msg = Message::Binary(bytes.into());

        for (id, client) in &self.clients {
            if Some(id) == exclude { continue; }
            match client {
                ClientType::ThreeDS { .. } => {
                    // UDP send handled separately
                }
                ClientType::WebSocket { tx, .. } => {
                    let _ = tx.send(msg.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    Join { session_id: Option<Uuid>, player_id: u8 },
    Action(ActionSync),
    Heartbeat,
    DeckSync(DeckSync),
    State { phase: String, turn: u32 },
    Error(String),
}

async fn handle_udp(
    socket: Arc<UdpSocket>,
    sessions: Arc<Mutex<HashMap<Uuid, Session>>>,
    cards: Arc<Vec<rabuka_engine::card::Card>>,
    decks: Arc<Vec<DeckList>>,
) {
    let mut buf = [0u8; MAX_PACKET];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                if len < 4 { continue; }
                let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                if magic != MAGIC { continue; }

                let payload = &buf[4..len];
                if let Ok(action) = serde_json::from_slice::<ActionSync>(payload) {
                    let mut sessions = sessions.lock().await;
                    // Find session by 3DS address
                    let session_id = sessions.iter()
                        .find(|(_, s)| s.three_ds_addr == Some(addr))
                        .map(|(id, _)| *id);

                    if let Some(id) = session_id {
                        if let Some(session) = sessions.get_mut(&id) {
                            session.last_activity = Instant::now();
                            // Execute action on local GameState
                            let mut gs = session.game_state.lock().await;
                            let action_type = rabuka_engine::game_setup::ActionType::from_tag(action.action_tag);
                            let stage_area = rabuka_engine::zones::MemberArea::from_tag(action.stage_area);
                            let _ = TurnEngine::execute_main_phase_action_with_ability_index(
                                &mut gs,
                                &action_type,
                                action.card_id,
                                if action.card_indices.is_empty() { None } else { Some(action.card_indices.clone()) },
                                stage_area,
                                Some(action.use_baton_touch),
                                action.ability_index.map(|x| x as usize),
                            );
                            gs.reset_loop_detection();

                            // Broadcast to WebSocket clients
                            drop(gs);
                            session.broadcast_action(&action, Some(&Uuid::nil())).await;

                            // Send ACK back to 3DS
                            let ack = serde_json::to_vec(&serde_json::json!({ "type": "ack", "seq": action.action_seq })).unwrap();
                            let mut ack_buf = BytesMut::with_capacity(4 + ack.len());
                            ack_buf.put_u32(MAGIC);
                            ack_buf.put_slice(&ack);
                            let _ = socket.send_to(&ack_buf, addr).await;
                        }
                    } else {
                        // First packet from new 3DS - could be deck sync
                        if let Ok(deck_sync) = serde_json::from_slice::<DeckSync>(payload) {
                            match Session::new(deck_sync.clone(), cards.clone(), &decks) {
                                Ok(mut session) => {
                                    session.three_ds_addr = Some(addr);
                                    let session_id = session.id;
                                    sessions.insert(session_id, session);
                                    info!("New 3DS session: {} from {}", session_id, addr);

                                    // Send deck sync ACK
                                    let ack = serde_json::to_vec(&serde_json::json!({ "type": "deck_sync_ack", "session_id": session_id })).unwrap();
                                    let mut ack_buf = BytesMut::with_capacity(4 + ack.len());
                                    ack_buf.put_u32(MAGIC);
                                    ack_buf.put_slice(&ack);
                                    let _ = socket.send_to(&ack_buf, addr).await;
                                }
                                Err(e) => {
                                    error!("Failed to create session: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("UDP recv error: {}", e);
            }
        }
    }
}

async fn handle_websocket(
    stream: tokio::net::TcpStream,
    sessions: Arc<Mutex<HashMap<Uuid, Session>>>,
    _cards: Arc<Vec<rabuka_engine::card::Card>>,
    _decks: Arc<Vec<DeckList>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (ws_tx, ws_rx) = ws_stream.split();
    let mut ws_tx = ws_tx;
    let mut ws_rx = ws_rx;
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let client_id = Uuid::new_v4();
    let mut joined_session: Option<Uuid> = None;
    let mut player_id: u8 = 0;

    // Spawn writer task
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() { break; }
        }
    });

    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                if let Ok(ws_msg) = serde_json::from_slice::<WsMessage>(&data) {
                    match ws_msg {
                        WsMessage::Join { session_id, player_id: pid } => {
                            player_id = pid;
                            let mut sessions = sessions.lock().await;
                            if let Some(id) = session_id {
                                if let Some(session) = sessions.get_mut(&id) {
                                    session.add_client(ClientType::WebSocket { id: client_id, player_id: pid, tx: tx.clone() });
                                    joined_session = Some(id);
                                    let _ = tx.send(Message::Binary(serde_json::to_vec(&WsMessage::State { phase: "joined".into(), turn: 0 }).unwrap().into()));
                                }
                            } else {
                                // Create new session or join existing
                                let _ = tx.send(Message::Binary(serde_json::to_vec(&WsMessage::Error("Session not found".into())).unwrap().into()));
                            }
                        }
                        WsMessage::Action(action) => {
                            if let Some(sid) = joined_session {
                                let mut sessions = sessions.lock().await;
                                if let Some(session) = sessions.get_mut(&sid) {
                                    session.last_activity = Instant::now();
                                    let mut gs = session.game_state.lock().await;
                                    let action_type = rabuka_engine::game_setup::ActionType::from_tag(action.action_tag);
                                    let stage_area = rabuka_engine::zones::MemberArea::from_tag(action.stage_area);
                                    let _ = TurnEngine::execute_main_phase_action_with_ability_index(
                                        &mut gs,
                                        &action_type,
                                        action.card_id,
                                        if action.card_indices.is_empty() { None } else { Some(action.card_indices.clone()) },
                                        stage_area,
                                        Some(action.use_baton_touch),
                                        action.ability_index.map(|x| x as usize),
                                    );
                                    gs.reset_loop_detection();
                                    drop(gs);
                                    session.broadcast_action(&action, Some(&client_id)).await;
                                }
                            }
                        }
                        WsMessage::Heartbeat => {}
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => { error!("WS error: {}", e); break; }
            _ => {}
        }
    }

    writer.abort();
    if let Some(sid) = joined_session {
        let mut sessions = sessions.lock().await;
        if let Some(session) = sessions.get_mut(&sid) {
            session.remove_client(&client_id);
        }
    }
}

async fn cleanup_sessions(sessions: Arc<Mutex<HashMap<Uuid, Session>>>) {
    let mut interval = interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        let mut sessions = sessions.lock().await;
        sessions.retain(|_, s| !s.is_expired());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Loading cards from {}", args.cards_path);
    let cards_json = std::fs::read_to_string(&args.cards_path)?;
    let cards_map: HashMap<String, rabuka_engine::card::Card> = serde_json::from_str(&cards_json)?;
    let cards: Arc<Vec<_>> = Arc::new(cards_map.into_values().collect());
    info!("Loaded {} cards", cards.len());

    let decks = DeckParser::parse_all_decks_from_directory(std::path::Path::new(&args.decks_path))
        .map_err(|e| anyhow::anyhow!(e))?;
    let decks: Arc<Vec<_>> = Arc::new(decks);
    info!("Loaded {} decks", decks.len());

    let sessions: Arc<Mutex<HashMap<Uuid, Session>>> = Arc::new(Mutex::new(HashMap::new()));

    let udp_socket = Arc::new(UdpSocket::bind(args.udp_addr).await?);
    info!("UDP listening on {}", args.udp_addr);

    let tcp_listener = TcpListener::bind(args.ws_addr).await?;
    info!("WebSocket listening on {}", args.ws_addr);

    let _udp_handle = tokio::spawn(handle_udp(udp_socket.clone(), sessions.clone(), cards.clone(), decks.clone()));
    let _cleanup_handle = tokio::spawn(cleanup_sessions(sessions.clone()));

    // Main accept loop
    loop {
        let (stream, addr) = tcp_listener.accept().await?;
        info!("WebSocket connection from {}", addr);
        tokio::spawn(handle_websocket(stream, sessions.clone(), cards.clone(), decks.clone()));
    }
}