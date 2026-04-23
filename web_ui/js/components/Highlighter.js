import { State } from '../state.js';
import { Tooltips } from '../ui_tooltips.js';

export const Highlighter = {
    addHighlight: (idOrEl, className) => {
        const el = typeof idOrEl === 'string' ? document.getElementById(idOrEl) : idOrEl;
        if (el) {
            el.classList.add(className);
            if (el.closest && el.closest('.card-area.hand')) {
                el.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
            }
        }
    },

    clearHighlights: () => {
        const selectors = [
            '.highlight-source', '.highlight-target', '.highlight-target-opp',
            '.valid-drop-target', '.drop-hover', '.highlight-hover',
            '.hover-highlight', '.selected', '.mulligan-selected'
        ];
        document.querySelectorAll(selectors.join(', ')).forEach(el => {
            el.classList.remove(
                'highlight-source', 'highlight-target', 'highlight-target-opp',
                'valid-drop-target', 'drop-hover', 'highlight-hover',
                'hover-highlight', 'selected', 'mulligan-selected'
            );
        });
    },

    highlightTargetsForAction: (action) => {
        if (!action) return;
        Highlighter.highlightAction(action);
    },

    highlightAction: (a) => {
        const state = State.data;
        if (!state) return;
        Highlighter.clearHighlights();

        const perspectivePlayer = State.perspectivePlayer;
        const actingPlayer = state.current_player ?? state.active_player;
        const selfPrefix = (actingPlayer === perspectivePlayer ? 'my' : 'opp');
        const oppPrefix = (actingPlayer === perspectivePlayer ? 'opp' : 'my');

        const getPlayerPrefix = (targetId) => {
            if (targetId === undefined) return selfPrefix;
            return (targetId === perspectivePlayer ? 'my' : 'opp');
        };

        const m = a.metadata || {};
        const targetPlayer = m.target_player;
        const targetPrefix = getPlayerPrefix(targetPlayer);

        let specificHighlighted = false;

        if (a.action_type === 'PlayMemberToStage' || a.type === 'PLAY') {
            const hIdx = a.hand_idx !== undefined ? a.hand_idx : a.parameters?.card_index;
            const sIdx = a.parameters?.stage_area ? (
                a.parameters.stage_area === 'left_side' ? 0 :
                a.parameters.stage_area === 'center' ? 1 :
                a.parameters.stage_area === 'right_side' ? 2 : undefined
            ) : a.area_idx;
            if (hIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-source');
                specificHighlighted = true;
            }
            if (sIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-stage-slot-${sIdx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'SetLiveCard' || a.type === 'LIVE_SET') {
            const hIdx = a.hand_idx !== undefined ? a.hand_idx : a.parameters?.card_index;
            if (hIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-source');
                specificHighlighted = true;
            }
            Highlighter.addHighlight(`${selfPrefix}-live`, 'highlight-target');
            specificHighlighted = true;
        } else if (a.action_type === 'UseAbility' || a.type === 'ABILITY' || m.category === 'ABILITY') {
            if (a.location === 'discard' || m.location === 'discard') {
                Highlighter.addHighlight(`${selfPrefix}-discard`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.parameters?.stage_area) {
                const sIdx = a.parameters.stage_area === 'left_side' ? 0 :
                             a.parameters.stage_area === 'center' ? 1 :
                             a.parameters.stage_area === 'right_side' ? 2 : undefined;
                if (sIdx !== undefined) {
                    Highlighter.addHighlight(`${selfPrefix}-stage-slot-${sIdx}`, 'highlight-source');
                    specificHighlighted = true;
                }
            } else if (a.parameters?.card_index !== undefined) {
                const hIdx = a.parameters.card_index;
                Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.area_idx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-stage-slot-${a.area_idx}`, 'highlight-source');
                specificHighlighted = true;
            } else if (a.slot_idx !== undefined) {
                Highlighter.addHighlight(`${targetPrefix}-stage-slot-${a.slot_idx}`, 'highlight-source');
                specificHighlighted = true;
            }
        } else if (a.type === 'CHOICE' || m.category === 'CHOICE') {
            const hIdx = a.hand_idx !== undefined ? a.hand_idx : a.parameters?.card_index;
            const sIdx = a.parameters?.stage_area ? (
                a.parameters.stage_area === 'left_side' ? 0 :
                a.parameters.stage_area === 'center' ? 1 :
                a.parameters.stage_area === 'right_side' ? 2 : undefined
            ) : a.area_idx ?? a.slot_idx;
            if (hIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-target');
                specificHighlighted = true;
            } else if (sIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-stage-slot-${sIdx}`, 'highlight-target');
                specificHighlighted = true;
            } else if (a.index !== undefined) {
                Highlighter.addHighlight(`select-list-item-${a.index}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'SelectMulligan' || a.action_type === 'MulliganHeader') {
            const hIdx = a.hand_idx !== undefined ? a.hand_idx : (a.parameters?.card_index);
            if (hIdx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'PlayMemberToStage' || a.action_type === 'UseAbility') {
            let hIdx = a.hand_idx ?? m.hand_idx;
            if (hIdx === undefined) {
                hIdx = a.parameters?.card_index;
            }
            if (hIdx !== undefined) {
                const id = `${targetPrefix}-hand-card-${hIdx}`;
                Highlighter.addHighlight(id, 'highlight-source');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'PlayMemberToStage' || a.action_type === 'Formation') {
            const idx = a.parameters?.stage_area ? (
                a.parameters.stage_area === 'left_side' ? 0 :
                a.parameters.stage_area === 'center' ? 1 :
                a.parameters.stage_area === 'right_side' ? 2 : undefined
            ) : (a.slot_idx ?? a.area_idx ?? m.slot_idx);
            if (idx !== undefined) {
                Highlighter.addHighlight(`${targetPrefix}-stage-slot-${idx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'SetLiveCard' || a.type === 'SELECT_LIVE') {
            const idx = a.parameters?.card_indices?.[0] ?? a.area_idx ?? a.slot_idx;
            if (idx !== undefined) {
                Highlighter.addHighlight(`${targetPrefix}-live-slot-${idx}`, 'highlight-target');
                specificHighlighted = true;
            }
        } else if (a.action_type === 'ActivateEnergy') {
            const idx = a.parameters?.card_index;
            if (idx !== undefined) {
                Highlighter.addHighlight(`${selfPrefix}-energy-slot-${idx}`, 'highlight-target');
                specificHighlighted = true;
            }
        }

        if (!specificHighlighted) {
            if (a.action_type === 'SELECT_DISCARD' || (a.metadata && (a.metadata.from_discard || a.metadata.category === 'DISCARD'))) {
                Highlighter.addHighlight(`${selfPrefix}-discard-visual`, 'highlight-target');
                specificHighlighted = true;
            } else if (a.action_type === 'SelectMulligan' || a.action_type === 'MulliganHeader') {
                const hIdx = a.parameters?.card_index;
                if (hIdx !== undefined && state.phase && state.phase.includes('Mulligan')) {
                    Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-target');
                    specificHighlighted = true;
                }
            } else if (a.action_type === 'SetLiveCard') {
                const hIdx = a.parameters?.card_index;
                if (hIdx !== undefined) {
                    Highlighter.addHighlight(`${selfPrefix}-hand-card-${hIdx}`, 'highlight-source');
                    Highlighter.addHighlight(`${selfPrefix}-live`, 'highlight-target');
                    specificHighlighted = true;
                }
            } else if (a.action_type === 'ChooseFirstAttacker' || a.action_type === 'ChooseSecondAttacker' || a.action_type === 'RockChoice' || a.action_type === 'PaperChoice' || a.action_type === 'ScissorsChoice') {
                // Do nothing - these are RPS-related
            } else {
                const slotIdx = a.slot_idx !== undefined ? a.slot_idx : (a.index !== undefined ? a.index : a.choice_idx);
                if (slotIdx !== undefined && slotIdx !== -1) {
                    if (a.action_type === 'SetLiveCard') {
                        Highlighter.addHighlight(`${selfPrefix}-live-slot-${slotIdx}`, 'highlight-target');
                    } else {
                        Highlighter.addHighlight(`${selfPrefix}-stage-slot-${slotIdx}`, 'highlight-target');
                    }
                }
                if (a.hand_idx !== undefined && a.hand_idx !== -1) {
                    Highlighter.addHighlight(`${selfPrefix}-hand-card-${a.hand_idx}`, 'highlight-target');
                }
                if (a.area_idx !== undefined && a.area_idx !== -1) {
                    const id = a.type === 'LIVE_SET' ? `${selfPrefix}-live-slot-${a.area_idx}` : `${selfPrefix}-stage-slot-${a.area_idx}`;
                    Highlighter.addHighlight(id, 'highlight-target');
                }
            }
        }

        if (!specificHighlighted) {
            let srcCardId = a.source_card_id;
            if ((srcCardId === undefined || srcCardId === -1) && state.pending_choice) {
                srcCardId = state.pending_choice.source_card_id || state.pending_choice.card_id || (state.pending_choice.params ? state.pending_choice.params.source_card_id : -1);
            }

            if (srcCardId !== undefined && srcCardId !== -1) {
                Highlighter.highlightCardById(srcCardId, 'highlight-source');
            }
        }
    },

    highlightPendingSource: () => {
        const state = State.data;
        if (!state || !state.pending_choice) return;
        const choice = state.pending_choice;
        const srcId = choice.source_card_id || choice.card_id || (choice.params ? choice.params.source_card_id : -1);

        if (srcId === undefined || srcId === -1) return;

        let found = false;
        const perspectivePlayer = State.perspectivePlayer;
        const selfPrefix = (state.active_player === perspectivePlayer ? 'my' : 'opp');

        const area = choice.area !== undefined ? choice.area : (choice.params ? choice.params.area : undefined);
        if (area !== undefined) {
            Highlighter.addHighlight(`${selfPrefix}-stage-slot-${area}`, 'highlight-source');
            found = true;
        }

        const handIdx = choice.hand_idx !== undefined ? choice.hand_idx : (choice.params ? choice.params.hand_idx : undefined);
        if (handIdx !== undefined) {
            Highlighter.addHighlight(`${selfPrefix}-hand-card-${handIdx}`, 'highlight-source');
            found = true;
        }

        if (!found) {
            Highlighter.highlightCardById(srcId);
        }
    },

    highlightCardById: (srcId, className = 'highlight-source', firstOnly = true) => {
        const state = State.data;
        if (!state) return;

        const perspectivePlayer = State.perspectivePlayer;
        const playersMap = [
            { id: perspectivePlayer, prefix: 'my' },
            { id: 1 - perspectivePlayer, prefix: 'opp' }
        ];

        for (const pMap of playersMap) {
            // Rust backend format: player1, player2
            const p = pMap.id === 0 ? state.player1 : state.player2;
            if (!p) continue;

            // Rust backend format: stage is { left_side, center, right_side }
            if (p.stage) {
                const stageCards = [p.stage.left_side, p.stage.center, p.stage.right_side].filter(c => c);
                for (let idx = 0; idx < stageCards.length; idx++) {
                    const card = stageCards[idx];
                    const cid = card ? card.card_no : -1;
                    if (cid === srcId) {
                        Highlighter.addHighlight(`${pMap.prefix}-stage-slot-${idx}`, className);
                        if (firstOnly) return;
                    }
                }
            }
            // Rust backend: hand is { cards: [...] }
            if (p.hand && p.hand.cards) {
                for (let idx = 0; idx < p.hand.cards.length; idx++) {
                    const card = p.hand.cards[idx];
                    const cid = card ? card.card_no : -1;
                    if (cid === srcId) {
                        Highlighter.addHighlight(`${pMap.prefix}-hand-card-${idx}`, className);
                        if (firstOnly) return;
                    }
                }
            }
            // Rust backend: live_zone is { cards: [...] }
            if (p.live_zone && p.live_zone.cards) {
                for (let idx = 0; idx < p.live_zone.cards.length; idx++) {
                    const cardObj = p.live_zone.cards[idx];
                    const cid = cardObj ? cardObj.card_no : -1;
                    if (cid === srcId) {
                        Highlighter.addHighlight(`${pMap.prefix}-live-slot-${idx}`, className);
                        if (firstOnly) return;
                    }
                }
            }
            if (p.discard && p.discard.some(c => (typeof c === 'object' ? c.card_no === srcId : c === srcId))) {
                Highlighter.addHighlight(`${pMap.prefix}-discard-visual`, className);
                if (firstOnly) return;
            }
            // Rust backend: energy is { cards: [...] }
            if (p.energy && p.energy.cards) {
                for (let idx = 0; idx < p.energy.cards.length; idx++) {
                    const e = p.energy.cards[idx];
                    const cid = e ? e.card_no : -1;
                    if (cid === srcId) {
                        Highlighter.addHighlight(`${pMap.prefix}-energy-slot-${idx}`, className);
                        if (firstOnly) return;
                    }
                }
            }
        }
    },

    highlightValidZones: (source, index) => {
        const state = State.data;
        if (!state || !state.legal_actions) return;

        const validTargets = new Set();
        const handIdx = index;

        state.legal_actions.forEach(a => {
            const params = a.parameters || {};
            if (source === 'hand') {
                if (params.card_index === handIdx) {
                    if (a.action_type === 'PlayMemberToStage' || a.action_type === 'Formation') {
                        if (params.stage_area) {
                            const areaMap = { 'left_side': 0, 'center': 1, 'right_side': 2 };
                            const stageIdx = areaMap[params.stage_area.toLowerCase()];
                            if (stageIdx !== undefined) validTargets.add(`my-stage-slot-${stageIdx}`);
                        }
                    }
                    if (a.action_type === 'LiveCardSet') {
                        if (params.card_indices) {
                            params.card_indices.forEach(idx => {
                                validTargets.add(`my-live-slot-${idx}`);
                            });
                        } else {
                            for (let i = 0; i < 3; i++) validTargets.add(`my-live-slot-${i}`);
                        }
                    }
                }
                if ((params.card_index === handIdx || params.card_indices?.includes(handIdx)) &&
                    (a.action_type === 'SelectHand' || a.description?.includes('Discard'))) {
                    validTargets.add('my-discard-visual');
                }
            } else if (source === 'stage') {
                const sourceSlot = index;
                if (params.stage_area) {
                    const areaMap = { 'left_side': 0, 'center': 1, 'right_side': 2 };
                    const stageIdx = areaMap[params.stage_area.toLowerCase()];
                    if (stageIdx !== undefined) validTargets.add(`opp-stage-slot-${stageIdx}`);
                }
            } else if (source === 'discard') {
                if (a.action_type === 'SelectDiscard' || a.action_type === 'SelectCard') {
                    validTargets.add('my-hand');
                }
            } else if (source === 'deck') {
                if (a.action_type === 'Draw') {
                    validTargets.add('my-hand');
                }
            }
        });

        validTargets.forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.classList.add('valid-drop-target');
                if (id.includes('slot-')) {
                    const container = el.closest('.board-slot-container');
                    if (container) container.classList.add('valid-drop-target');
                }
            }
        });
    },

    highlightStageCard: (areaIdx) => {
        Highlighter.clearHighlights();
        Highlighter.addHighlight(`my-stage-slot-${areaIdx}`, 'highlight-source');
    }
};
