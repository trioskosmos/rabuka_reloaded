import { State } from '../state.js';
import { fixImg } from '../constants.js';
import { Network } from '../network.js';

// Metadata Constants for Debug Display (mirrors data/metadata.json)
const DEBUG_CONSTANTS = {
    // Ability Effect Flags
    FLAG_DRAW: 1,
    FLAG_SEARCH: 2,
    FLAG_RECOVER: 4,
    FLAG_BUFF: 8,
    FLAG_CHARGE: 16,
    FLAG_TEMPO: 32,
    FLAG_REDUCE: 64,
    FLAG_BOOST: 128,
    FLAG_TRANSFORM: 256,
    FLAG_WIN_COND: 512,
    FLAG_MOVE: 1024,
    FLAG_TAP: 2048,
    // Cost Flags
    COST_FLAG_DISCARD: 1,
    COST_FLAG_TAP: 2,
    // Choice Flags
    CHOICE_FLAG_LOOK: 1,
    CHOICE_FLAG_DISCARD: 2,
    CHOICE_FLAG_MODE: 4,
    CHOICE_FLAG_COLOR: 8,
    CHOICE_FLAG_ORDER: 16,
    // Synergy Flags
    SYN_FLAG_GROUP: 1,
    SYN_FLAG_COLOR: 2,
    SYN_FLAG_BATON: 4,
    SYN_FLAG_CENTER: 8,
    SYN_FLAG_LIFE_LEAD: 16,
    // Filter Flags (partial)
    FILTER_TYPE_MEMBER: 4,
    FILTER_TYPE_LIVE: 8,
    FILTER_GROUP_ENABLE: 16,
    FILTER_TAPPED: 4096,
    FILTER_HAS_BLADE_HEART: 8192,
    FILTER_NOT_HAS_BLADE_HEART: 16384,
    FILTER_UNIQUE_NAMES: 32768,
    FILTER_UNIT_ENABLE: 65536,
    // Area Constants
    AREA_LEFT: 1,
    AREA_CENTER: 2,
    AREA_RIGHT: 3,
    // Zones
    ZONE_MASK_STAGE: 4,
    ZONE_MASK_HAND: 6,
    ZONE_MASK_DISCARD: 7,
};

export const DebugModal = {
    // State for filters
    _filters: {
        selectedPlayer: 0,
        selectedZone: 'all', // 'all', 'stage', 'live', 'hand', 'success', 'energy', 'discard'
        filterByAbility: true,
        abilitySearch: '',
        showBytecodeAlways: true,
    },

    init: () => { },

    openDebugModal: () => {
        const modal = document.getElementById('debug-modal');
        if (modal) {
            modal.style.display = 'flex';
            if (!State.data) {
                const containers = ['debug-inspector-content', 'debug-bytecode-log'];
                containers.forEach(id => {
                    const el = document.getElementById(id);
                    if (el) {
                        el.innerHTML = '<div class="debug-loading">Waiting for game state...</div>';
                    }
                });
                return;
            }
            DebugModal.renderAll();
        }
    },

    closeDebugModal: () => {
        const modal = document.getElementById('debug-modal');
        if (modal) modal.style.display = 'none';
    },

    switchTab: (tabId) => {
        const contents = document.querySelectorAll('.debug-inspector-tab');
        contents.forEach(c => { c.style.display = 'none'; });

        const selected = document.getElementById(`debug-tab-${tabId}`);
        if (selected) {
            selected.style.display = 'block';
        }

        const buttons = document.querySelectorAll('.debug-tab-btn');
        buttons.forEach(b => {
            if (b.getAttribute('data-tab') === tabId) {
                b.classList.add('active');
                b.style.borderBottom = '2px solid var(--accent-blue)';
                b.style.color = '#fff';
            } else {
                b.classList.remove('active');
                b.style.borderBottom = '2px solid transparent';
                b.style.color = 'var(--text-dim)';
            }
        });

        if (tabId === 'string') DebugModal.renderStringTab();
        if (tabId === 'json') DebugModal.renderJsonTab();
        if (tabId === 'inspector') DebugModal.renderInspectorTab();
    },

    renderAll: async () => {
        if (State.roomCode) { await Network.fetchState(); }
        if (!State.data) return;
        try { DebugModal.renderBytecodeLog(); } catch (e) { console.error("Debug Bytecode Render Fail:", e); }
        try { DebugModal.renderInspectorTab(); } catch (e) { console.error("Debug Inspector Error:", e); }
        try { DebugModal.renderJsonTab(); } catch (e) { console.error("Debug JSON Error:", e); }
        try { DebugModal.renderStringTab(); } catch (e) { console.error("Debug String Error:", e); }
    },

    renderStringTab: () => {
        const container = document.getElementById('debug-tab-string');
        if (!container || !State.data) return;

        // Generate current state string using standardized payload
        let blob = "";
        let stateSize = 0;
        try {
            // Use standardized state payload (same as bug report)
            const source = State.stripRichData(State.data);
            const jsonStr = JSON.stringify(source);
            stateSize = jsonStr.length;
            blob = btoa(unescape(encodeURIComponent(jsonStr)));
        } catch (e) {
            blob = "Error generating blob: " + e.message;
        }

        container.innerHTML = `
            <div style="display: flex; flex-direction: column; height: 100%; padding: 15px; gap: 12px;">
                <div style="font-size: 12px; line-height: 1.5; opacity: 0.9; background: rgba(52,152,219,0.1); padding: 10px; border-radius: 4px; border-left: 3px solid var(--accent-blue);">
                    <p style="margin: 0 0 6px 0;"><strong>Game State Checkpoint (Base64-Encoded)</strong></p>
                    <p style="margin: 0 0 6px 0; opacity: 0.8; font-size: 11px;">
                        Compact serialization of the full game state. Copy to save, paste and load to restore this exact board state.
                    </p>
                    <p style="margin: 0; opacity: 0.7; font-size: 10px; font-family: monospace;">
                        Raw: ${(stateSize / 1024).toFixed(2)} KB | Base64: ${blob.length} chars
                    </p>
                </div>
                <textarea id="debug-string-textarea"
                    style="flex: 1; background: #0a0a0a; color: #00ff00; border: 1px solid #333; border-radius: 4px; padding: 10px; font-family: 'Cascadia Code', monospace; font-size: 11px; resize: none; word-break: break-all; white-space: pre-wrap;"
                    spellcheck="false">${blob}</textarea>
                <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                    <button class="btn btn-primary" style="flex: 1; min-width: 120px;" data-action="debug-copy-state-string">📋 Copy</button>
                    <button class="btn btn-secondary" style="flex: 1; min-width: 120px;" data-action="debug-load-state-string">📁 Load</button>
                    <button class="btn btn-accent" style="flex: 1; min-width: 120px; background: var(--accent-gold); color: #000;" data-action="debug-trigger-file-load">📂 File</button>
                    <input type="file" id="debug-state-file-input" style="display: none;" accept=".json,.txt,.b64" onchange="DebugModal.loadStateFile(this)">
                </div>
            </div>
        `;
    },

    triggerFileLoad: () => {
        const input = document.getElementById('debug-state-file-input');
        if (input) input.click();
    },

    loadStateFile: (input) => {
        if (!input.files || !input.files[0]) return;
        const file = input.files[0];
        const reader = new FileReader();
        reader.onload = async (e) => {
            try {
                let json;
                // Try to detect if it's base64-encoded state string or JSON
                const content = e.target.result.trim();

                // Check if it looks like base64 (contains typical base64 chars and likely starts with { when decoded)
                if (!/^[\s\n\r{[]/.test(content) && /^[A-Za-z0-9+/=]+$/.test(content)) {
                    // Looks like base64, decode it
                    const decoded = JSON.parse(decodeURIComponent(escape(atob(content))));
                    json = decoded;
                } else {
                    // Try to parse as JSON directly
                    json = JSON.parse(content);
                }

                // Strip if it's rich data
                const source = State.stripRichData(json);
                const ok = await Network.applyState(JSON.stringify(source));
                if (ok) {
                    alert("State Loaded from File and Applied Successfully.");
                    DebugModal.renderAll();
                } else {
                    alert("Apply failed — check server console.");
                }
            } catch (err) {
                alert("File Read/Parse Error: " + err.message);
            }
            // Reset input so the same file can be picked again
            input.value = "";
        };
        reader.readAsText(file);
    },

    _flag: (label, val, color = '#ccc') => {
        if (val === undefined || val === null) return '';
        const display = typeof val === 'boolean' ? (val ? 'YES' : 'NO') : val;
        const c = (typeof val === 'boolean' && val) ? '#e74c3c' : (typeof val === 'boolean' ? '#2ecc71' : color);
        return `<div style="font-size:11px;"><span style="opacity:0.5;">${label}:</span> <span style="color:${c}">${display}</span></div>`;
    },

    _renderLogicList: (title, items, color) => {
        if (!items || items.length === 0) return '';
        return `
            <div style="margin-top:5px; border-left: 2px solid ${color}; padding-left: 8px;">
                <div style="font-size: 9px; opacity: 0.5; text-transform: uppercase;">${title}</div>
                ${items.map(item => {
            const typeName = item.condition_type || item.effect_type || item.cost_type || item.type || 'Unknown';
            return `
                    <div style="font-size: 10px; margin-bottom: 2px;">
                        <span style="color:${color}">${typeName}:</span>
                        ${item.value !== undefined ? `v=${item.value}` : ''}
                        ${item.attr !== undefined ? `a=0x${Number(item.attr).toString(16)}` : ''}
                        ${item.target !== undefined ? `t=${item.target}` : ''}
                        ${item.is_negated ? '<span style="color:#e74c3c">[NEG]</span>' : ''}
                        ${item.is_optional ? '<span style="color:#3498db">[OPT]</span>' : ''}
                    </div>`;
        }).join('')}
            </div>
        `;
    },

    _renderSlotEnums: (card) => {
        if (!card || card.id === -1 || card.id === -2) return '';

        const triggers = new Set();
        const conditions = new Set();
        const effects = new Set();
        const costs = new Set();

        const triggerMap = {0:'NONE',1:'ON_PLAY',2:'ON_LIVE_START',3:'ON_LIVE_SUCCESS',4:'TURN_START',5:'TURN_END',6:'CONSTANT',7:'ACTIVATED',8:'ON_LEAVES',9:'ON_REVEAL',10:'ON_POSITION_CHANGE',11:'ON_ABILITY_RESOLVE',12:'ON_ABILITY_SUCCESS'};
        const conditionMap = {200:'TURN_1',201:'HAS_MEMBER',202:'HAS_COLOR',203:'COUNT_STAGE',204:'COUNT_HAND',205:'COUNT_DISCARD',206:'IS_CENTER',207:'LIFE_LEAD',208:'COUNT_GROUP',209:'GROUP_FILTER',210:'OPPONENT_HAS',211:'SELF_IS_GROUP',212:'MODAL_ANSWER',213:'COUNT_ENERGY',214:'HAS_LIVE_CARD',215:'COST_CHECK',216:'RARITY_CHECK',217:'HAND_HAS_NO_LIVE',218:'COUNT_SUCCESS_LIVE',219:'OPPONENT_HAND_DIFF',220:'SCORE_COMPARE',221:'HAS_CHOICE',222:'OPPONENT_CHOICE',223:'COUNT_HEARTS',224:'COUNT_BLADES',225:'OPPONENT_ENERGY_DIFF',226:'HAS_KEYWORD',227:'DECK_REFRESHED',228:'HAS_MOVED',229:'HAND_INCREASED',230:'COUNT_LIVE_ZONE',231:'BATON',232:'TYPE_CHECK',233:'IS_IN_DISCARD',234:'AREA_CHECK',235:'COST_LEAD',236:'SCORE_LEAD',237:'HEART_LEAD',238:'HAS_EXCESS_HEART',239:'NOT_HAS_EXCESS_HEART',240:'TOTAL_BLADES',241:'COST_COMPARE',242:'BLADE_COMPARE',243:'HEART_COMPARE',244:'OPPONENT_HAS_WAIT',245:'IS_TAPPED',246:'IS_ACTIVE',247:'LIVE_PERFORMED',248:'IS_PLAYER',249:'IS_OPPONENT',250:'COUNT_UNIQUE_COLORS',301:'COUNT_ENERGY_EXACT',302:'COUNT_BLADE_HEART_TYPES',303:'OPPONENT_HAS_EXCESS_HEART',304:'SCORE_TOTAL_CHECK',305:'MAIN_PHASE',306:'SELECT_MEMBER',307:'SUCCESS_PILE_COUNT',308:'IS_SELF_MOVE',309:'DISCARDED_CARDS',310:'YELL_REVEALED_UNIQUE_COLORS',311:'SYNC_COST',312:'SUM_VALUE',313:'IS_WAIT',314:'ON_ABILITY_RESOLVE',315:'TARGET_MEMBER_HAS_NO_HEARTS'};
        const effectMap = {0:'NOP',1:'RETURN',2:'JUMP',3:'JUMP_IF_FALSE',10:'DRAW',11:'ADD_BLADES',12:'ADD_HEARTS',13:'REDUCE_COST',14:'LOOK_DECK',15:'RECOVER_LIVE',16:'BOOST_SCORE',17:'RECOVER_MEMBER',18:'BUFF_POWER',19:'IMMUNITY',20:'MOVE_MEMBER',21:'SWAP_CARDS',22:'SEARCH_DECK',23:'ENERGY_CHARGE',24:'SET_BLADES',25:'SET_HEARTS',26:'FORMATION_CHANGE',27:'NEGATE_EFFECT',28:'ORDER_DECK',29:'META_RULE',30:'SELECT_MODE',31:'MOVE_TO_DECK',32:'TAP_OPPONENT',33:'PLACE_UNDER',34:'FLAVOR_ACTION',35:'RESTRICTION',36:'BATON_TOUCH_MOD',37:'SET_SCORE',38:'SWAP_ZONE',39:'TRANSFORM_COLOR',40:'REVEAL_CARDS',41:'LOOK_AND_CHOOSE',42:'CHEER_REVEAL',43:'ACTIVATE_MEMBER',44:'ADD_TO_HAND',45:'COLOR_SELECT',47:'TRIGGER_REMOTE',48:'REDUCE_HEART_REQ',49:'MODIFY_SCORE_RULE',50:'ADD_STAGE_ENERGY',51:'SET_TAPPED',53:'TAP_MEMBER',57:'PLAY_MEMBER_FROM_HAND',58:'MOVE_TO_DISCARD',60:'GRANT_ABILITY',61:'INCREASE_HEART_COST',62:'REDUCE_YELL_COUNT',63:'PLAY_MEMBER_FROM_DISCARD',64:'PAY_ENERGY',65:'SELECT_MEMBER',66:'DRAW_UNTIL',67:'SELECT_PLAYER',68:'SELECT_LIVE',69:'REVEAL_UNTIL',70:'INCREASE_COST',71:'PREVENT_PLAY_TO_SLOT',72:'SWAP_AREA',73:'TRANSFORM_HEART',74:'SELECT_CARDS',75:'OPPONENT_CHOOSE',76:'PLAY_LIVE_FROM_DISCARD',77:'REDUCE_LIVE_SET_LIMIT',78:'SET_TARGET_SELF',79:'SET_TARGET_OPPONENT',80:'PREVENT_SET_TO_SUCCESS_PILE',81:'ACTIVATE_ENERGY',82:'PREVENT_ACTIVATE',83:'SET_HEART_COST',90:'PREVENT_BATON_TOUCH',91:'LOOK_DECK_DYNAMIC',92:'REDUCE_SCORE',93:'REPEAT_ABILITY',94:'LOSE_EXCESS_HEARTS',95:'SKIP_ACTIVATE_PHASE',96:'PAY_ENERGY_DYNAMIC',97:'PLACE_ENERGY_UNDER_MEMBER',106:'CALC_SUM_COST',125:'LOOK_REORDER_DISCARD',126:'DIV_VALUE',127:'TRANSFORM_BLADES'};
        const costMap = {0:'NONE',1:'ENERGY',2:'TAP_SELF',3:'DISCARD_HAND',4:'RETURN_HAND',5:'SACRIFICE_SELF',8:'DISCARD_ENERGY',20:'TAP_MEMBER',21:'TAP_ENERGY',22:'REST_MEMBER',23:'RETURN_MEMBER_TO_HAND',24:'DISCARD_MEMBER',25:'DISCARD_LIVE',26:'REMOVE_LIVE',27:'REMOVE_MEMBER',31:'PLACE_MEMBER_FROM_HAND',32:'PLACE_LIVE_FROM_HAND',33:'PLACE_ENERGY_FROM_HAND',34:'PLACE_MEMBER_FROM_DISCARD',35:'PLACE_LIVE_FROM_DISCARD',36:'PLACE_ENERGY_FROM_DISCARD',37:'PLACE_MEMBER_FROM_DECK',38:'PLACE_LIVE_FROM_DECK',39:'PLACE_ENERGY_FROM_DECK'};

        if (card.abilities) {
            card.abilities.forEach(ab => {
                if (ab.trigger !== undefined) triggers.add(triggerMap[ab.trigger] || ab.trigger);
                if (ab.conditions) ab.conditions.forEach(c => {
                    if (c.condition_type !== undefined) conditions.add(conditionMap[c.condition_type] || c.condition_type);
                });
                if (ab.effects) ab.effects.forEach(e => {
                    if (e.effect_type !== undefined) effects.add(effectMap[e.effect_type] || e.effect_type);
                });
                if (ab.costs) ab.costs.forEach(c => {
                    if (c.cost_type !== undefined) costs.add(costMap[c.cost_type] || c.cost_type);
                });
            });
        }

        if (!triggers.size && !conditions.size && !effects.size && !costs.size) return '';

        return `
            <div style="margin-top: 6px; padding: 6px; background: rgba(255,255,255,0.02); border-radius: 3px; border: 1px dashed rgba(255,255,255,0.1);">
                <div style="font-size: 9px; font-weight: bold; text-transform: uppercase; opacity: 0.6; margin-bottom: 3px;">Enums on This Card:</div>
                ${triggers.size ? `<div style="font-size: 8px; margin-bottom: 2px;"><strong style="color:#2ecc71;">Triggers:</strong> ${Array.from(triggers).join(', ')}</div>` : ''}
                ${conditions.size ? `<div style="font-size: 8px; margin-bottom: 2px;"><strong style="color:#3498db;">Conditions:</strong> ${Array.from(conditions).join(', ')}</div>` : ''}
                ${effects.size ? `<div style="font-size: 8px; margin-bottom: 2px;"><strong style="color:#f39c12;">Effects:</strong> ${Array.from(effects).join(', ')}</div>` : ''}
                ${costs.size ? `<div style="font-size: 8px;"><strong style="color:#1abc9c;">Cost Types:</strong> ${Array.from(costs).join(', ')}</div>` : ''}
            </div>
        `;
    },

    _renderCardDiag: (c, slotLabel) => {
        if (!c || c === -1 || c.id === -1 || c.id === -2) {
            return `<div style="font-size:11px; opacity:0.3; padding:8px; border:1px dashed #444; border-radius:4px; text-align:center;">EMPTY (${slotLabel})</div>`;
        }
        const flags = [];
        if (c.tapped) flags.push('<span style="color:#e74c3c">TAPPED</span>');
        if (c.moved) flags.push('<span style="color:#3498db">MOVED</span>');
        if (c.revealed) flags.push('<span style="color:#2ecc71">REVEALED</span>');

        const type = c.type || (c.score !== undefined ? 'live' : 'member');
        const isLive = type === 'live';

        return `
            <div style="font-size:11px; background: rgba(255,255,255,0.03); padding: 10px; border: 1px solid #444; border-radius: 4px; display:flex; flex-direction:column; gap:6px;">
                <div style="display:flex; justify-content:space-between; border-bottom:1px solid rgba(255,255,255,0.1); padding-bottom:4px;">
                    <div style="display:flex; align-items:center; gap:8px;">
                        <strong style="color:${isLive ? '#e74c3c' : 'var(--accent-blue)'}; font-size:12px;">${c.name}</strong>
                        <span style="opacity:0.5; font-size:10px;">[${slotLabel}]</span>
                        <span style="opacity:0.4; font-size:9px; background:rgba(255,255,255,0.1); padding:0 4px; border-radius:3px;">${type.toUpperCase()}</span>
                    </div>
                    <span style="opacity:0.5; font-family:monospace; font-size:10px;">ID ${c.id}</span>
                </div>

                <div style="display:grid; grid-template-columns: repeat(auto-fit, minmax(80px, 1fr)); gap:6px; background: rgba(0,0,0,0.2); padding:4px 6px; border-radius:4px; font-size:10px;">
                    ${isLive ? `<span><strong>Score:</strong> ${c.score ?? 0}</span>` : `<span><strong>Cost:</strong> ${c.cost ?? 0}</span>`}
                    ${isLive ? `<span><strong>Req:</strong> ${JSON.stringify(c.required_hearts || [])}</span>` : `<span><strong>Hearts:</strong> ${JSON.stringify(c.hearts || [])}</span>`}
                    ${!isLive ? `<span><strong>Blades:</strong> ${c.blades ?? 0}</span>` : ''}
                    <span><strong>Vol:</strong> ${c.note_icons ?? 0}</span>
                </div>

                <div style="display:flex; flex-wrap:wrap; gap:8px; font-size:9px;">
                    <div title="Semantic Flags">Sem: <span style="color:#3498db">0x${(c.semantic_flags || 0).toString(16)}</span></div>
                    ${!isLive ? `<div title="Ability Flags">Ab: <span style="color:#2ecc71">0x${(c.ability_flags || 0).toString(16)}</span></div>` : ''}
                    <div title="Synergy Flags">Syn: <span style="color:#f1c40f">0x${(c.synergy_flags || 0).toString(16)}</span></div>
                    ${!isLive ? `<div title="Cost Flags">CostF: <span style="color:#9b59b6">0x${(c.cost_flags || 0).toString(16)}</span></div>` : ''}
                </div>

                ${flags.length ? `<div style="display:flex; flex-wrap:wrap; gap:4px; font-size:9px; font-weight:bold;">${flags.join(' ')}</div>` : ''}

                <div class="debug-abilities" style="margin-top:4px; border-top:1px solid rgba(255,255,255,0.05); padding-top:6px; display:flex; flex-direction:column; gap:10px;">
                    ${(c.abilities || []).map((ab, ai) => `
                        <div style="background: rgba(255,255,255,0.02); padding:6px; border-radius:4px; border: 1px solid rgba(255,255,255,0.05);">
                            <div style="display:flex; justify-content:space-between; margin-bottom:4px;">
                                <div style="color:#f1c40f; font-size:10px; font-weight:bold;">Ab#${ai + 1} [${ab.trigger}]</div>
                                <div style="display:flex; gap:4px;">
                                    ${ab.is_once_per_turn ? '<span style="font-size:8px; color:#e67e22; border:1px solid #e67e22; padding:0 3px; border-radius:3px;">1/TURN</span>' : ''}
                                    ${ab.requires_selection ? '<span style="font-size:8px; color:#9b59b6; border:1px solid #9b59b6; padding:0 3px; border-radius:3px;">CHOICE</span>' : ''}
                                    ${ab.choice_count > 0 ? `<span style="font-size:8px; color:#3498db; border:1px solid #3498db; padding:0 3px; border-radius:3px;">x${ab.choice_count}</span>` : ''}
                                </div>
                            </div>
                            <div style="opacity:0.8; font-style:italic; margin-bottom:6px; font-size:10px; border-bottom: 1px dashed rgba(255,255,255,0.1); padding-bottom:4px;">"${ab.pseudocode || 'No pseudocode'}"</div>

                            ${DebugModal._renderLogicList('Conditions', ab.conditions, '#3498db')}
                            ${DebugModal._renderLogicList('Costs', ab.costs, '#e67e22')}
                            ${DebugModal._renderLogicList('Effects', ab.effects, '#2ecc71')}

                            ${(ab.decoded_bytecode && ab.decoded_bytecode.length > 0) ? `
                            <details style="margin-top:4px;">
                                <summary style="cursor:pointer; opacity:0.5; font-size:9px; padding:2px; background:rgba(255,255,255,0.05); border-radius:3px;">Decoded Bytecode (${ab.decoded_bytecode.length})</summary>
                                <pre style="font-size:9px; background:#000; padding:6px; border-radius:4px; margin:4px 0; max-height:150px; overflow-y:auto; color:#0f0; font-family: 'Cascadia Code', monospace; line-height:1.2;">${ab.decoded_bytecode.join('\n')}</pre>
                            </details>` : ''}

                            ${(ab.bytecode && ab.bytecode.length > 0) ? `
                            <details style="margin-top:2px;">
                                <summary style="cursor:pointer; opacity:0.4; font-size:8px; padding:1px 2px; background:rgba(255,255,255,0.03); border-radius:3px;">Raw Bytecode (${ab.bytecode.length} words)</summary>
                                <pre style="font-size:8px; background:#000; padding:4px; border-radius:4px; margin:2px 0; max-height:80px; overflow-y:auto; color:#888; font-family:monospace;">${ab.bytecode.join(', ')}</pre>
                            </details>` : ''}
                        </div>
                    `).join('')}
                </div>

                ${DebugModal._renderSlotEnums(c)}
            </div>
        `;
    },

    // Render constants reference section
    _renderConstantsRef: () => {
        const C = DEBUG_CONSTANTS;
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Constants Reference</summary>
                <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:9px;">
                    <div style="grid-column: span 4; font-weight:bold; color:#f1c40f; border-bottom:1px solid #444; margin-bottom:4px;">Ability Flags</div>
                    ${Object.entries({ FLAG_DRAW: 1, FLAG_SEARCH: 2, FLAG_RECOVER: 4, FLAG_BUFF: 8, FLAG_CHARGE: 16, FLAG_TEMPO: 32, FLAG_REDUCE: 64, FLAG_BOOST: 128, FLAG_TRANSFORM: 256, FLAG_WIN_COND: 512, FLAG_MOVE: 1024, FLAG_TAP: 2048 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                    <div style="grid-column: span 4; font-weight:bold; color:#e67e22; border-bottom:1px solid #444; margin-bottom:4px; margin-top:4px;">Cost Flags</div>
                    ${Object.entries({ COST_FLAG_DISCARD: 1, COST_FLAG_TAP: 2 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                    <div style="grid-column: span 4; font-weight:bold; color:#3498db; border-bottom:1px solid #444; margin-bottom:4px; margin-top:4px;">Choice Flags</div>
                    ${Object.entries({ CHOICE_FLAG_LOOK: 1, CHOICE_FLAG_DISCARD: 2, CHOICE_FLAG_MODE: 4, CHOICE_FLAG_COLOR: 8, CHOICE_FLAG_ORDER: 16 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                    <div style="grid-column: span 4; font-weight:bold; color:#9b59b6; border-bottom:1px solid #444; margin-bottom:4px; margin-top:4px;">Synergy Flags</div>
                    ${Object.entries({ SYN_FLAG_GROUP: 1, SYN_FLAG_COLOR: 2, SYN_FLAG_BATON: 4, SYN_FLAG_CENTER: 8, SYN_FLAG_LIFE_LEAD: 16 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                    <div style="grid-column: span 4; font-weight:bold; color:#2ecc71; border-bottom:1px solid #444; margin-bottom:4px; margin-top:4px;">Filter Flags</div>
                    ${Object.entries({ FILTER_TYPE_MEMBER: 4, FILTER_TYPE_LIVE: 8, FILTER_GROUP_ENABLE: 16, FILTER_TAPPED: 4096, FILTER_HAS_BLADE_HEART: 8192, FILTER_NOT_HAS_BLADE_HEART: 16384, FILTER_UNIQUE_NAMES: 32768, FILTER_UNIT_ENABLE: 65536 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                    <div style="grid-column: span 4; font-weight:bold; color:#e74c3c; border-bottom:1px solid #444; margin-bottom:4px; margin-top:4px;">Area/Zones</div>
                    ${Object.entries({ AREA_LEFT: 1, AREA_CENTER: 2, AREA_RIGHT: 3, ZONE_MASK_STAGE: 4, ZONE_MASK_HAND: 6, ZONE_MASK_DISCARD: 7 }).map(([k, v]) => `<span>${k}:${v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render conditions reference
    _renderConditionsRef: () => {
        const conditions = [
            { n: 'TURN_1', v: 200 }, { n: 'HAS_MEMBER', v: 201 }, { n: 'HAS_COLOR', v: 202 }, { n: 'COUNT_STAGE', v: 203 }, { n: 'COUNT_HAND', v: 204 }, { n: 'COUNT_DISCARD', v: 205 }, { n: 'IS_CENTER', v: 206 }, { n: 'LIFE_LEAD', v: 207 }, { n: 'COUNT_GROUP', v: 208 }, { n: 'GROUP_FILTER', v: 209 }, { n: 'OPPONENT_HAS', v: 210 }, { n: 'SELF_IS_GROUP', v: 211 }, { n: 'MODAL_ANSWER', v: 212 }, { n: 'COUNT_ENERGY', v: 213 }, { n: 'HAS_LIVE_CARD', v: 214 }, { n: 'COST_CHECK', v: 215 }, { n: 'RARITY_CHECK', v: 216 }, { n: 'HAND_HAS_NO_LIVE', v: 217 }, { n: 'COUNT_SUCCESS_LIVE', v: 218 }, { n: 'OPPONENT_HAND_DIFF', v: 219 }, { n: 'SCORE_COMPARE', v: 220 }, { n: 'HAS_CHOICE', v: 221 }, { n: 'OPPONENT_CHOICE', v: 222 }, { n: 'COUNT_HEARTS', v: 223 }, { n: 'COUNT_BLADES', v: 224 }, { n: 'OPPONENT_ENERGY_DIFF', v: 225 }, { n: 'HAS_KEYWORD', v: 226 }, { n: 'DECK_REFRESHED', v: 227 }, { n: 'HAS_MOVED', v: 228 }, { n: 'HAND_INCREASED', v: 229 }, { n: 'COUNT_LIVE_ZONE', v: 230 }, { n: 'BATON', v: 231 }, { n: 'TYPE_CHECK', v: 232 }, { n: 'IS_IN_DISCARD', v: 233 }, { n: 'AREA_CHECK', v: 234 }, { n: 'COST_LEAD', v: 235 }, { n: 'SCORE_LEAD', v: 236 }, { n: 'HEART_LEAD', v: 237 }, { n: 'HAS_EXCESS_HEART', v: 238 }, { n: 'NOT_HAS_EXCESS_HEART', v: 239 }, { n: 'TOTAL_BLADES', v: 240 }, { n: 'COST_COMPARE', v: 241 }, { n: 'BLADE_COMPARE', v: 242 }, { n: 'HEART_COMPARE', v: 243 }, { n: 'OPPONENT_HAS_WAIT', v: 244 }, { n: 'IS_TAPPED', v: 245 }, { n: 'IS_ACTIVE', v: 246 }, { n: 'LIVE_PERFORMED', v: 247 }, { n: 'IS_PLAYER', v: 248 }, { n: 'IS_OPPONENT', v: 249 }, { n: 'COUNT_UNIQUE_COLORS', v: 250 }, { n: 'COUNT_ENERGY_EXACT', v: 301 }, { n: 'COUNT_BLADE_HEART_TYPES', v: 302 }, { n: 'OPPONENT_HAS_EXCESS_HEART', v: 303 }, { n: 'SCORE_TOTAL_CHECK', v: 304 }, { n: 'MAIN_PHASE', v: 305 }, { n: 'SELECT_MEMBER', v: 306 }, { n: 'SUCCESS_PILE_COUNT', v: 307 }, { n: 'IS_SELF_MOVE', v: 308 }, { n: 'DISCARDED_CARDS', v: 309 }, { n: 'YELL_REVEALED_UNIQUE_COLORS', v: 310 }, { n: 'SYNC_COST', v: 311 }, { n: 'SUM_VALUE', v: 312 }, { n: 'IS_WAIT', v: 313 }, { n: 'ON_ABILITY_RESOLVE', v: 314 }, { n: 'TARGET_MEMBER_HAS_NO_HEARTS', v: 315 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Conditions Reference (${conditions.length})</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:8px; max-height:250px; overflow-y:auto;">
                    ${conditions.map(c => `<span style="color:#3498db;">${c.n}:${c.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render phases reference
    _renderPhasesRef: () => {
        const phases = [
            { n: 'SETUP', v: -4 }, { n: 'RPS', v: -3 }, { n: 'TURN_CHOICE', v: -2 }, { n: 'MULLIGAN_P1', v: -1 }, { n: 'MULLIGAN_P2', v: 0 }, { n: 'ACTIVE', v: 1 }, { n: 'ENERGY', v: 2 }, { n: 'DRAW', v: 3 }, { n: 'MAIN', v: 4 }, { n: 'LIVE_SET', v: 5 }, { n: 'PERFORMANCE_P1', v: 6 }, { n: 'PERFORMANCE_P2', v: 7 }, { n: 'LIVE_RESULT', v: 8 }, { n: 'TERMINAL', v: 9 }, { n: 'RESPONSE', v: 10 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Phases Reference</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:9px;">
                    ${phases.map(p => `<span style="color:#f1c40f;">${p.n}:${p.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render triggers reference
    _renderTriggersRef: () => {
        const triggers = [
            { n: 'NONE', v: 0 }, { n: 'ON_PLAY', v: 1 }, { n: 'ON_LIVE_START', v: 2 }, { n: 'ON_LIVE_SUCCESS', v: 3 }, { n: 'TURN_START', v: 4 }, { n: 'TURN_END', v: 5 }, { n: 'CONSTANT', v: 6 }, { n: 'ACTIVATED', v: 7 }, { n: 'ON_LEAVES', v: 8 }, { n: 'ON_REVEAL', v: 9 }, { n: 'ON_POSITION_CHANGE', v: 10 }, { n: 'ON_ABILITY_RESOLVE', v: 11 }, { n: 'ON_ABILITY_SUCCESS', v: 12 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Triggers Reference</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:9px;">
                    ${triggers.map(t => `<span style="color:#2ecc71;">${t.n}:${t.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render target types reference
    _renderTargetTypesRef: () => {
        const targets = [
            { n: 'SELF', v: 0 }, { n: 'PLAYER', v: 1 }, { n: 'OPPONENT', v: 2 }, { n: 'ALL_PLAYERS', v: 3 }, { n: 'MEMBER_SELF', v: 4 }, { n: 'MEMBER_OTHER', v: 5 }, { n: 'CARD_HAND', v: 6 }, { n: 'CARD_DISCARD', v: 7 }, { n: 'CARD_DECK_TOP', v: 8 }, { n: 'OPPONENT_HAND', v: 9 }, { n: 'MEMBER_SELECT', v: 10 }, { n: 'MEMBER_NAMED', v: 11 }, { n: 'OPPONENT_MEMBER', v: 12 }, { n: 'PLAYER_SELECT', v: 20 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Target Types Reference</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:9px;">
                    ${targets.map(t => `<span style="color:#9b59b6;">${t.n}:${t.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render effect types reference
    _renderEffectTypesRef: () => {
        const effects = [
            { n: 'NOP', v: 0 }, { n: 'RETURN', v: 1 }, { n: 'JUMP', v: 2 }, { n: 'JUMP_IF_FALSE', v: 3 }, { n: 'DRAW', v: 10 }, { n: 'ADD_BLADES', v: 11 }, { n: 'ADD_HEARTS', v: 12 }, { n: 'REDUCE_COST', v: 13 }, { n: 'LOOK_DECK', v: 14 }, { n: 'RECOVER_LIVE', v: 15 }, { n: 'BOOST_SCORE', v: 16 }, { n: 'RECOVER_MEMBER', v: 17 }, { n: 'BUFF_POWER', v: 18 }, { n: 'IMMUNITY', v: 19 }, { n: 'MOVE_MEMBER', v: 20 }, { n: 'SWAP_CARDS', v: 21 }, { n: 'SEARCH_DECK', v: 22 }, { n: 'ENERGY_CHARGE', v: 23 }, { n: 'SET_BLADES', v: 24 }, { n: 'SET_HEARTS', v: 25 }, { n: 'FORMATION_CHANGE', v: 26 }, { n: 'NEGATE_EFFECT', v: 27 }, { n: 'ORDER_DECK', v: 28 }, { n: 'META_RULE', v: 29 }, { n: 'SELECT_MODE', v: 30 }, { n: 'MOVE_TO_DECK', v: 31 }, { n: 'TAP_OPPONENT', v: 32 }, { n: 'PLACE_UNDER', v: 33 }, { n: 'FLAVOR_ACTION', v: 34 }, { n: 'RESTRICTION', v: 35 }, { n: 'BATON_TOUCH_MOD', v: 36 }, { n: 'SET_SCORE', v: 37 }, { n: 'SWAP_ZONE', v: 38 }, { n: 'TRANSFORM_COLOR', v: 39 }, { n: 'REVEAL_CARDS', v: 40 }, { n: 'LOOK_AND_CHOOSE', v: 41 }, { n: 'CHEER_REVEAL', v: 42 }, { n: 'ACTIVATE_MEMBER', v: 43 }, { n: 'ADD_TO_HAND', v: 44 }, { n: 'COLOR_SELECT', v: 45 }, { n: 'TRIGGER_REMOTE', v: 47 }, { n: 'REDUCE_HEART_REQ', v: 48 }, { n: 'MODIFY_SCORE_RULE', v: 49 }, { n: 'ADD_STAGE_ENERGY', v: 50 }, { n: 'SET_TAPPED', v: 51 }, { n: 'TAP_MEMBER', v: 53 }, { n: 'PLAY_MEMBER_FROM_HAND', v: 57 }, { n: 'MOVE_TO_DISCARD', v: 58 }, { n: 'GRANT_ABILITY', v: 60 }, { n: 'INCREASE_HEART_COST', v: 61 }, { n: 'REDUCE_YELL_COUNT', v: 62 }, { n: 'PLAY_MEMBER_FROM_DISCARD', v: 63 }, { n: 'PAY_ENERGY', v: 64 }, { n: 'SELECT_MEMBER', v: 65 }, { n: 'DRAW_UNTIL', v: 66 }, { n: 'SELECT_PLAYER', v: 67 }, { n: 'SELECT_LIVE', v: 68 }, { n: 'REVEAL_UNTIL', v: 69 }, { n: 'INCREASE_COST', v: 70 }, { n: 'PREVENT_PLAY_TO_SLOT', v: 71 }, { n: 'SWAP_AREA', v: 72 }, { n: 'TRANSFORM_HEART', v: 73 }, { n: 'SELECT_CARDS', v: 74 }, { n: 'OPPONENT_CHOOSE', v: 75 }, { n: 'PLAY_LIVE_FROM_DISCARD', v: 76 }, { n: 'REDUCE_LIVE_SET_LIMIT', v: 77 }, { n: 'SET_TARGET_SELF', v: 78 }, { n: 'SET_TARGET_OPPONENT', v: 79 }, { n: 'PREVENT_SET_TO_SUCCESS_PILE', v: 80 }, { n: 'ACTIVATE_ENERGY', v: 81 }, { n: 'PREVENT_ACTIVATE', v: 82 }, { n: 'SET_HEART_COST', v: 83 }, { n: 'PREVENT_BATON_TOUCH', v: 90 }, { n: 'LOOK_DECK_DYNAMIC', v: 91 }, { n: 'REDUCE_SCORE', v: 92 }, { n: 'REPEAT_ABILITY', v: 93 }, { n: 'LOSE_EXCESS_HEARTS', v: 94 }, { n: 'SKIP_ACTIVATE_PHASE', v: 95 }, { n: 'PAY_ENERGY_DYNAMIC', v: 96 }, { n: 'PLACE_ENERGY_UNDER_MEMBER', v: 97 }, { n: 'CALC_SUM_COST', v: 106 }, { n: 'LOOK_REORDER_DISCARD', v: 125 }, { n: 'DIV_VALUE', v: 126 }, { n: 'TRANSFORM_BLADES', v: 127 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Effect Types Reference (${effects.length})</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:8px; max-height:250px; overflow-y:auto;">
                    ${effects.map(e => `<span style="color:#f39c12;">${e.n}:${e.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render ability cost types reference
    _renderAbilityCostTypesRef: () => {
        const costs = [
            { n: 'NONE', v: 0 }, { n: 'ENERGY', v: 1 }, { n: 'TAP_SELF', v: 2 }, { n: 'DISCARD_HAND', v: 3 }, { n: 'RETURN_HAND', v: 4 }, { n: 'SACRIFICE_SELF', v: 5 }, { n: 'REVEAL_HAND_ALL', v: 6 }, { n: 'SACRIFICE_UNDER', v: 7 }, { n: 'DISCARD_ENERGY', v: 8 }, { n: 'REVEAL_HAND', v: 9 }, { n: 'TAP_MEMBER', v: 20 }, { n: 'TAP_ENERGY', v: 21 }, { n: 'REST_MEMBER', v: 22 }, { n: 'RETURN_MEMBER_TO_HAND', v: 23 }, { n: 'DISCARD_MEMBER', v: 24 }, { n: 'DISCARD_LIVE', v: 25 }, { n: 'REMOVE_LIVE', v: 26 }, { n: 'REMOVE_MEMBER', v: 27 }, { n: 'RETURN_LIVE_TO_HAND', v: 28 }, { n: 'RETURN_LIVE_TO_DECK', v: 29 }, { n: 'RETURN_MEMBER_TO_DECK', v: 30 }, { n: 'PLACE_MEMBER_FROM_HAND', v: 31 }, { n: 'PLACE_LIVE_FROM_HAND', v: 32 }, { n: 'PLACE_ENERGY_FROM_HAND', v: 33 }, { n: 'PLACE_MEMBER_FROM_DISCARD', v: 34 }, { n: 'PLACE_LIVE_FROM_DISCARD', v: 35 }, { n: 'PLACE_ENERGY_FROM_DISCARD', v: 36 }, { n: 'PLACE_MEMBER_FROM_DECK', v: 37 }, { n: 'PLACE_LIVE_FROM_DECK', v: 38 }, { n: 'PLACE_ENERGY_FROM_DECK', v: 39 }, { n: 'SHUFFLE_DECK', v: 41 }, { n: 'DRAW_CARD', v: 42 }, { n: 'DISCARD_TOP_DECK', v: 43 }, { n: 'REMOVE_TOP_DECK', v: 44 }, { n: 'RETURN_DISCARD_TO_DECK', v: 45 }, { n: 'RETURN_REMOVED_TO_DECK', v: 46 }, { n: 'RETURN_REMOVED_TO_HAND', v: 47 }, { n: 'RETURN_REMOVED_TO_DISCARD', v: 48 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Cost Types Reference (${costs.length})</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:8px; max-height:250px; overflow-y:auto;">
                    ${costs.map(c => `<span style="color:#1abc9c;">${c.n}:${c.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    // Render choice types reference
    _renderChoiceTypesRef: () => {
        const choices = [
            { n: 'NONE', v: 0 }, { n: 'OPTIONAL', v: 1 }, { n: 'PAY_ENERGY', v: 2 }, { n: 'REVEAL_HAND', v: 3 }, { n: 'SELECT_DISCARD', v: 4 }, { n: 'SELECT_SWAP_SOURCE', v: 5 }, { n: 'SELECT_STAGE', v: 6 }, { n: 'SELECT_STAGE_EMPTY', v: 7 }, { n: 'SELECT_LIVE_SLOT', v: 8 }, { n: 'SELECT_SWAP_TARGET', v: 9 }, { n: 'SELECT_MEMBER', v: 10 }, { n: 'SELECT_DISCARD_PLAY', v: 11 }, { n: 'SELECT_HAND_DISCARD', v: 12 }, { n: 'COLOR_SELECT', v: 13 }, { n: 'SELECT_MODE', v: 14 }, { n: 'OPPONENT_CHOOSE', v: 15 }, { n: 'SELECT_CARDS_ORDER', v: 16 }, { n: 'TAP_O', v: 17 }, { n: 'LOOK_AND_CHOOSE', v: 18 }, { n: 'SELECT_CARDS', v: 19 }, { n: 'SELECT_PLAYER', v: 20 }, { n: 'SELECT_LIVE', v: 21 }, { n: 'ORDER_DECK', v: 22 }, { n: 'SELECT_HAND_PLAY', v: 23 }, { n: 'TAP_M_SELECT', v: 24 }, { n: 'MOVE_MEMBER_DEST', v: 25 }, { n: 'RECOV_L', v: 26 }, { n: 'RECOV_M', v: 27 }, { n: 'SELECT_STAGE_EMPTY_BATON', v: 28 }, { n: 'REARRANGE_FORMATION', v: 29 }
        ];
        return `
            <details style="margin-top:8px;">
                <summary style="cursor:pointer; font-size:10px; font-weight:bold; text-transform:uppercase; opacity:0.6; letter-spacing:1px;">Choice Types Reference (${choices.length})</summary>
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 2px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-top:4px; font-size:9px;">
                    ${choices.map(c => `<span style="color:#e74c3c;">${c.n}:${c.v}</span>`).join('')}
                </div>
            </details>
        `;
    },

    renderFlags: () => {
        const container = document.getElementById('debug-flags-content');
        if (!container || !State.data) return;

        const d = State.data;
        const F = DebugModal._flag;

        let html = `
            <div class="debug-grid" style="display: flex; flex-direction: column; gap: 16px; padding-bottom: 20px;">
                ${DebugModal._renderActiveEnumsSection()}

                <div style="background: rgba(255,255,255,0.05); padding: 12px; border-radius: 8px; border: 1px solid var(--border);">
                    <h3 style="margin-top:0; color: var(--accent-blue); display:flex; justify-content:space-between;">
                        <span>System Engine Tracer</span>
                        <span style="font-size: 11px; opacity: 0.5;">v2.0-deep-diag</span>
                    </h3>
                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; font-size: 12px;">
                        ${F('Phase', `${d.phase}`, '#f1c40f')}
                        ${F('Turn', d.turn)}
                        ${F('Active', `P${(d.active_player ?? 0) + 1}`, '#3498db')}
                        ${F('Queue', d.queue_depth || 0)}
                        ${F('RPS', JSON.stringify(d.rps_choices || []))}
                        ${F('Winner', d.winner !== undefined && d.winner !== -1 ? 'P' + (d.winner + 1) : 'NONE')}
                        ${F('FirstPl', `P${(d.first_player ?? 0) + 1}`)}
                        ${F('TrigDepth', d.trigger_depth || 0)}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; font-size: 11px; margin-top: 8px; border-top: 1px solid rgba(255,255,255,0.05); padding-top: 8px;">
                        ${F('InteractDepth', d.pending_choice ? 1 : 0)}
                        ${F('LiveResPend', d.live_result_selection_pending ? 'YES' : 'NO')}
                        ${F('NeedsDeck', d.needs_deck ? 'YES' : 'NO')}
                        ${F('Spectators', d.spectators || 0)}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; font-size: 11px; margin-top: 8px; border-top: 1px solid rgba(255,255,255,0.05); padding-top: 8px;">
                        ${F('PrevPhase', d.prev_phase ?? '-')}
                        ${F('PrevCard', d.prev_card_id ?? -1)}
                        ${F('LiveSetDraws', JSON.stringify(d.live_set_pending_draws || [0, 0]))}
                        ${F('LiveResTrigDone', d.live_result_triggers_done ? 'YES' : 'NO')}
                        ${F('LiveStartTrigDone', d.live_start_triggers_done ? 'YES' : 'NO')}
                        ${F('ScoreReqPlayer', d.score_req_player ?? -1)}
                        ${F('ScoreReqList', JSON.stringify(d.score_req_list || []))}
                    </div>
                </div>
        `;

        d.players.forEach((p, i) => {
            const isViewer = (i === State.perspectivePlayer);
            const hearts = p.total_hearts || [0, 0, 0, 0, 0, 0, 0];

            html += `
                <div style="background: rgba(0,0,0,0.2); padding: 12px; border-radius: 8px; border: 1px solid var(--border);">
                    <h4 style="margin-top:0; border-bottom: 1px solid var(--border); padding-bottom: 5px; display:flex; justify-content:space-between;">
                        <span>Player ${i + 1} ${isViewer ? '<span style="font-size:10px; opacity:0.6;">(VIEWER)</span>' : ''}</span>
                        <span style="font-size:12px; color:var(--accent-blue);">Score: ${p.score}</span>
                    </h4>

                    <!-- RESOURCES -->
                    <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; margin-bottom: 10px;">
                        ${F('Energy', `${p.energy_untapped ?? 0}/${p.energy_count ?? 0}`)}
                        ${F('Hand', p.hand?.length || 0)}
                        ${F('Deck', p.deck_count ?? 0)}
                        ${F('Discard', p.discard_count ?? 0)}
                        ${F('EnergyDeck', p.energy_deck_count ?? 0)}
                        ${F('SuccessLives', p.success_lives?.length || 0)}
                        ${F('LiveZone', (p.live_zone || []).filter(c => c && c.id !== -1).length)}
                        ${F('YellCards', p.yell_cards?.length || 0)}
                        ${F('Exile', p.exile_count || 0)}
                        ${F('LiveDeck', p.live_deck_count || 0)}
                    </div>

                    <!-- HEARTS BAR -->
                    <div style="display: flex; gap: 2px; margin-bottom: 10px;">
                        ${hearts.map((h, hi) => `<div style="flex:1; height:20px; background:var(--color-${hi}); color:#000; font-size:10px; font-weight:bold; display:flex; align-items:center; justify-content:center; border-radius:2px;" title="Color ${hi}">${h}</div>`).join('')}
                    </div>

                    <!-- ALL FLAGS -->
                    <details open>
                        <summary style="cursor:pointer; font-size:11px; font-weight:bold; text-transform:uppercase; opacity:0.7; letter-spacing:1px; margin-bottom:6px;">Engine Flags & Conditions</summary>
                        <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-bottom:10px;">
                            ${F('CostReduction', p.cost_reduction ?? 0)}
                            ${F('BatonCount', `${p.baton_touch_count ?? 0}/${p.baton_touch_limit ?? 3}`)}
                            ${F('PrevActivate', p.prevent_activate, p.prevent_activate ? '#e74c3c' : '#2ecc71')}
                            ${F('PrevBaton', p.prevent_baton_touch, p.prevent_baton_touch ? '#e74c3c' : '#2ecc71')}
                            ${F('PrevSuccess', p.prevent_success_pile_set, p.prevent_success_pile_set ? '#e74c3c' : '#2ecc71')}
                            ${F('PrevPlaySlotMask', `0x${(p.prevent_play_to_slot_mask || 0).toString(16)}`)}
                            ${F('SkipNextAct', p.skip_next_activate)}
                            ${F('LiveScoreBonus', p.live_score_bonus ?? 0)}
                            ${F('YellReduction', p.yell_count_reduction ?? 0)}
                            ${F('CheerMod', p.cheer_mod_count ?? 0)}
                            ${F('PlayCount', p.play_count_this_turn ?? 0)}
                            ${F('HandIncrTurn', p.hand_increased_this_turn ?? 0)}
                            ${F('DiscardedTurn', p.discarded_this_turn ?? 0)}
                            ${F('TurnVolume', p.current_turn_notes ?? 0)}
                            ${F('ExcessHearts', p.excess_hearts ?? 0)}
                            ${F('FlagsBits', `0x${(p.flags || 0).toString(16).toUpperCase()}`)}
                            ${F('PlayedGrpMask', `0x${(p.played_group_mask || 0).toString(16).toUpperCase()}`)}
                            ${F('ActEnergyGrp', `0x${(p.activated_energy_group_mask || 0).toString(16).toUpperCase()}`)}
                            ${F('ActMemberGrp', `0x${(p.activated_member_group_mask || 0).toString(16).toUpperCase()}`)}
                            ${F('ColorXforms', p.color_transforms_count ?? 0)}
                            ${F('NegatedTrigs', p.negated_triggers_count ?? 0)}
                            ${F('GrantedAbs', p.granted_abilities_count ?? 0)}
                            ${F('UsedAbs', p.used_abilities_count ?? 0)}
                            ${F('Restrictions', JSON.stringify(p.restrictions || []))}
                            ${F('MullSelection', `0x${(p.mulligan_selection || 0).toString(16).toUpperCase()}`)}
                            ${F('ObtSuccess', p.obtained_success_live ? 'YES' : 'NO')}
                            ${F('LvRevealed', JSON.stringify(p.live_zone_revealed || [false, false, false]))}
                            <!-- NEW: Additional Missing Fields -->
                            ${F('TappedEnergyMask', `0x${(p.tapped_energy_mask || 0).toString(16).toUpperCase()}`)}
                            ${F('CostMods', JSON.stringify(p.cost_modifiers || []))}
                            ${F('BladeBuffLogs', JSON.stringify(p.blade_buff_logs || []))}
                            ${F('HeartBuffLogs', JSON.stringify(p.heart_buff_logs || []))}
                            ${F('HandAddedTurn', JSON.stringify(p.hand_added_turn || []))}
                            ${F('PerfTrigAbs', JSON.stringify(p.perf_triggered_abilities || []))}
                            <!-- NEW: Flags Bit Breakdown -->
                            <div style="grid-column: span 4; display: flex; gap: 4px; flex-wrap: wrap; margin-top: 4px; padding: 4px; background: rgba(0,0,0,0.3); border-radius: 4px;">
                                <span style="font-size:9px; color: ${(p.flags || 0) & 1 ? '#e74c3c' : '#2ecc71'}">CANNOT_LIVE:${(p.flags || 0) & 1 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 2 ? '#e74c3c' : '#2ecc71'}">DECK_REFRESHED:${(p.flags || 0) & 2 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 4 ? '#e74c3c' : '#2ecc71'}">IMMUNITY:${(p.flags || 0) & 4 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 8 ? '#e74c3c' : '#2ecc71'}">TAPPED_M0:${(p.flags || 0) & 8 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 16 ? '#e74c3c' : '#2ecc71'}">TAPPED_M1:${(p.flags || 0) & 16 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 32 ? '#e74c3c' : '#2ecc71'}">TAPPED_M2:${(p.flags || 0) & 32 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 64 ? '#e74c3c' : '#2ecc71'}">MOVED_M0:${(p.flags || 0) & 64 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 128 ? '#e74c3c' : '#2ecc71'}">MOVED_M1:${(p.flags || 0) & 128 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 256 ? '#e74c3c' : '#2ecc71'}">MOVED_M2:${(p.flags || 0) & 256 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 512 ? '#e74c3c' : '#2ecc71'}">LIVE_REV0:${(p.flags || 0) & 512 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 1024 ? '#e74c3c' : '#2ecc71'}">LIVE_REV1:${(p.flags || 0) & 1024 ? 'Y' : 'N'}</span>
                                <span style="font-size:9px; color: ${(p.flags || 0) & 2048 ? '#e74c3c' : '#2ecc71'}">LIVE_REV2:${(p.flags || 0) & 2048 ? 'Y' : 'N'}</span>
                            </div>
                        </div>
                    </details>

                    <!-- PER-SLOT DATA -->
                    <details open>
                        <summary style="cursor:pointer; font-size:11px; font-weight:bold; text-transform:uppercase; opacity:0.7; letter-spacing:1px; margin-bottom:6px;">Per-Slot Buffs & Energy</summary>
                        <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-bottom:10px;">
                            ${[0, 1, 2].map(s => `
                                <div style="padding:4px; border:1px solid #333; border-radius:4px; font-size:10px;">
                                    <div style="font-weight:bold; margin-bottom:3px;">Slot ${s + 1}</div>
                                    ${F('BladeBuffs', (p.blade_buffs || [])[s] ?? 0)}
                                    ${F('HeartBuffs', JSON.stringify((p.heart_buffs || [])[s] || []))}
                                    ${F('CostMod', (p.slot_cost_modifiers || [])[s] ?? 0)}
                                    ${F('StgEnergy', (p.stage_energy_count || [])[s] ?? 0)}
                                    ${F('StgEnergyCards', JSON.stringify((p.stage_energy || [])[s] || []))}
                                </div>
                            `).join('')}
                        </div>
                    </details>

                    ${(p.heart_req_reductions && p.heart_req_reductions.some(v => v > 0)) || (p.heart_req_additions && p.heart_req_additions.some(v => v > 0)) ? `
                    <details>
                        <summary style="cursor:pointer; font-size:11px; font-weight:bold; text-transform:uppercase; opacity:0.7; letter-spacing:1px; margin-bottom:6px;">Heart Requirement Mods</summary>
                        <div style="background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-bottom:10px; font-size:10px;">
                            ${F('Reductions', JSON.stringify(p.heart_req_reductions || []))}
                            ${F('Additions', JSON.stringify(p.heart_req_additions || []))}
                        </div>
                    </details>` : ''}

                    ${(p.live_score_bonus_logs && p.live_score_bonus_logs.length > 0) ? `
                    <details>
                        <summary style="cursor:pointer; font-size:11px; font-weight:bold; text-transform:uppercase; opacity:0.7; letter-spacing:1px; margin-bottom:6px;">Score Bonus Logs (${p.live_score_bonus_logs.length})</summary>
                        <div style="background:rgba(0,0,0,0.2); padding:8px; border-radius:4px; margin-bottom:10px; font-size:10px;">
                            ${p.live_score_bonus_logs.map(l => `<div>Source: ${l.source}, Amount: +${l.amount}</div>`).join('')}
                        </div>
                    </details>` : ''}

                    <!-- STAGE DIAGNOSTICS -->
                    <h5 style="margin: 8px 0 6px 0; font-size: 11px; text-transform: uppercase; opacity: 0.7; letter-spacing: 1px;">Stage Cards</h5>
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        ${(p.stage || []).map((c, si) => DebugModal._renderCardDiag(c, `Stage ${si + 1}`)).join('')}
                    </div>

                    <!-- LIVE ZONE DIAGNOSTICS -->
                    <h5 style="margin: 12px 0 6px 0; font-size: 11px; text-transform: uppercase; opacity: 0.7; letter-spacing: 1px;">Live Set Slots</h5>
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        ${(p.live_zone || []).map((c, li) => DebugModal._renderCardDiag(c, `Live ${li + 1}`)).join('')}
                    </div>
                </div>
            `;
        });

        html += '</div>';

        // Add Constants Reference Section at the end
        html += `
            <div style="background: rgba(0,0,0,0.2); padding: 12px; border-radius: 8px; border: 1px solid var(--border);">
                <h4 style="margin-top:0; color: var(--accent-gold);">Metadata Constants Reference</h4>
                ${DebugModal._renderConstantsRef()}
                ${DebugModal._renderConditionsRef()}
                ${DebugModal._renderPhasesRef()}
                ${DebugModal._renderTriggersRef()}
                ${DebugModal._renderTargetTypesRef()}
                ${DebugModal._renderEffectTypesRef()}
                ${DebugModal._renderAbilityCostTypesRef()}
                ${DebugModal._renderChoiceTypesRef()}
            </div>
        `;

        container.innerHTML = html;
    },

    _analyzeActiveEnums: () => {
        if (!State.data || !State.data.players) return { triggers: [], conditions: [], effects: [], costs: [], choices: [] };

        const activeTriggers = new Set();
        const activeConditions = new Set();
        const activeEffects = new Set();
        const activeCosts = new Set();
        const activeChoices = new Set();

        // Trigger Type Mappings
        const triggerMap = {0:'NONE',1:'ON_PLAY',2:'ON_LIVE_START',3:'ON_LIVE_SUCCESS',4:'TURN_START',5:'TURN_END',6:'CONSTANT',7:'ACTIVATED',8:'ON_LEAVES',9:'ON_REVEAL',10:'ON_POSITION_CHANGE',11:'ON_ABILITY_RESOLVE',12:'ON_ABILITY_SUCCESS'};

        // Condition Type Mappings
        const conditionMap = {200:'TURN_1',201:'HAS_MEMBER',202:'HAS_COLOR',203:'COUNT_STAGE',204:'COUNT_HAND',205:'COUNT_DISCARD',206:'IS_CENTER',207:'LIFE_LEAD',208:'COUNT_GROUP',209:'GROUP_FILTER',210:'OPPONENT_HAS',211:'SELF_IS_GROUP',212:'MODAL_ANSWER',213:'COUNT_ENERGY',214:'HAS_LIVE_CARD',215:'COST_CHECK',216:'RARITY_CHECK',217:'HAND_HAS_NO_LIVE',218:'COUNT_SUCCESS_LIVE',219:'OPPONENT_HAND_DIFF',220:'SCORE_COMPARE',221:'HAS_CHOICE',222:'OPPONENT_CHOICE',223:'COUNT_HEARTS',224:'COUNT_BLADES',225:'OPPONENT_ENERGY_DIFF',226:'HAS_KEYWORD',227:'DECK_REFRESHED',228:'HAS_MOVED',229:'HAND_INCREASED',230:'COUNT_LIVE_ZONE',231:'BATON',232:'TYPE_CHECK',233:'IS_IN_DISCARD',234:'AREA_CHECK',235:'COST_LEAD',236:'SCORE_LEAD',237:'HEART_LEAD',238:'HAS_EXCESS_HEART',239:'NOT_HAS_EXCESS_HEART',240:'TOTAL_BLADES',241:'COST_COMPARE',242:'BLADE_COMPARE',243:'HEART_COMPARE',244:'OPPONENT_HAS_WAIT',245:'IS_TAPPED',246:'IS_ACTIVE',247:'LIVE_PERFORMED',248:'IS_PLAYER',249:'IS_OPPONENT',250:'COUNT_UNIQUE_COLORS',301:'COUNT_ENERGY_EXACT',302:'COUNT_BLADE_HEART_TYPES',303:'OPPONENT_HAS_EXCESS_HEART',304:'SCORE_TOTAL_CHECK',305:'MAIN_PHASE',306:'SELECT_MEMBER',307:'SUCCESS_PILE_COUNT',308:'IS_SELF_MOVE',309:'DISCARDED_CARDS',310:'YELL_REVEALED_UNIQUE_COLORS',311:'SYNC_COST',312:'SUM_VALUE',313:'IS_WAIT',314:'ON_ABILITY_RESOLVE',315:'TARGET_MEMBER_HAS_NO_HEARTS'};

        // Effect Type Mappings (partial key set for common ones)
        const effectMap = {0:'NOP',1:'RETURN',2:'JUMP',3:'JUMP_IF_FALSE',10:'DRAW',11:'ADD_BLADES',12:'ADD_HEARTS',13:'REDUCE_COST',14:'LOOK_DECK',15:'RECOVER_LIVE',16:'BOOST_SCORE',17:'RECOVER_MEMBER',18:'BUFF_POWER',19:'IMMUNITY',20:'MOVE_MEMBER',21:'SWAP_CARDS',22:'SEARCH_DECK',23:'ENERGY_CHARGE',24:'SET_BLADES',25:'SET_HEARTS',26:'FORMATION_CHANGE',27:'NEGATE_EFFECT',28:'ORDER_DECK',29:'META_RULE',30:'SELECT_MODE',31:'MOVE_TO_DECK',32:'TAP_OPPONENT',33:'PLACE_UNDER',34:'FLAVOR_ACTION',35:'RESTRICTION',36:'BATON_TOUCH_MOD',37:'SET_SCORE',38:'SWAP_ZONE',39:'TRANSFORM_COLOR',40:'REVEAL_CARDS',41:'LOOK_AND_CHOOSE',42:'CHEER_REVEAL',43:'ACTIVATE_MEMBER',44:'ADD_TO_HAND',45:'COLOR_SELECT',47:'TRIGGER_REMOTE',48:'REDUCE_HEART_REQ',49:'MODIFY_SCORE_RULE',50:'ADD_STAGE_ENERGY',51:'SET_TAPPED',53:'TAP_MEMBER',57:'PLAY_MEMBER_FROM_HAND',58:'MOVE_TO_DISCARD',60:'GRANT_ABILITY',61:'INCREASE_HEART_COST',62:'REDUCE_YELL_COUNT',63:'PLAY_MEMBER_FROM_DISCARD',64:'PAY_ENERGY',65:'SELECT_MEMBER',66:'DRAW_UNTIL',67:'SELECT_PLAYER',68:'SELECT_LIVE',69:'REVEAL_UNTIL',70:'INCREASE_COST',71:'PREVENT_PLAY_TO_SLOT',72:'SWAP_AREA',73:'TRANSFORM_HEART',74:'SELECT_CARDS',75:'OPPONENT_CHOOSE',76:'PLAY_LIVE_FROM_DISCARD',77:'REDUCE_LIVE_SET_LIMIT',78:'SET_TARGET_SELF',79:'SET_TARGET_OPPONENT',80:'PREVENT_SET_TO_SUCCESS_PILE',81:'ACTIVATE_ENERGY',82:'PREVENT_ACTIVATE',83:'SET_HEART_COST',90:'PREVENT_BATON_TOUCH',91:'LOOK_DECK_DYNAMIC',92:'REDUCE_SCORE',93:'REPEAT_ABILITY',94:'LOSE_EXCESS_HEARTS',95:'SKIP_ACTIVATE_PHASE',96:'PAY_ENERGY_DYNAMIC',97:'PLACE_ENERGY_UNDER_MEMBER',106:'CALC_SUM_COST',125:'LOOK_REORDER_DISCARD',126:'DIV_VALUE',127:'TRANSFORM_BLADES'};

        // Cost Type Mappings (partial for key ones)
        const costMap = {0:'NONE',1:'ENERGY',2:'TAP_SELF',3:'DISCARD_HAND',4:'RETURN_HAND',5:'SACRIFICE_SELF',8:'DISCARD_ENERGY',20:'TAP_MEMBER',21:'TAP_ENERGY',22:'REST_MEMBER',23:'RETURN_MEMBER_TO_HAND',24:'DISCARD_MEMBER',25:'DISCARD_LIVE',26:'REMOVE_LIVE',27:'REMOVE_MEMBER',31:'PLACE_MEMBER_FROM_HAND',32:'PLACE_LIVE_FROM_HAND',33:'PLACE_ENERGY_FROM_HAND',34:'PLACE_MEMBER_FROM_DISCARD',35:'PLACE_LIVE_FROM_DISCARD',36:'PLACE_ENERGY_FROM_DISCARD',37:'PLACE_MEMBER_FROM_DECK',38:'PLACE_LIVE_FROM_DECK',39:'PLACE_ENERGY_FROM_DECK'};

        // Choice Type Mappings
        const choiceMap = {0:'NONE',1:'OPTIONAL',2:'PAY_ENERGY',3:'REVEAL_HAND',4:'SELECT_DISCARD',5:'SELECT_SWAP_SOURCE',6:'SELECT_STAGE',7:'SELECT_STAGE_EMPTY',8:'SELECT_LIVE_SLOT',9:'SELECT_SWAP_TARGET',10:'SELECT_MEMBER',11:'SELECT_DISCARD_PLAY',12:'SELECT_HAND_DISCARD',13:'COLOR_SELECT',14:'SELECT_MODE',15:'OPPONENT_CHOOSE',16:'SELECT_CARDS_ORDER',17:'TAP_O',18:'LOOK_AND_CHOOSE',19:'SELECT_CARDS',20:'SELECT_PLAYER',21:'SELECT_LIVE',22:'ORDER_DECK',23:'SELECT_HAND_PLAY',24:'TAP_M_SELECT',25:'MOVE_MEMBER_DEST',26:'RECOV_L',27:'RECOV_M',28:'SELECT_STAGE_EMPTY_BATON',29:'REARRANGE_FORMATION'};

        // Analyze abilities from all stages and live zones
        State.data.players.forEach((p, pi) => {
            // Stage cards
            (p.stage || []).forEach(card => {
                if (card && card.abilities) {
                    card.abilities.forEach(ab => {
                        if (ab.trigger !== undefined) activeTriggers.add(`${triggerMap[ab.trigger] || ab.trigger}`);
                        if (ab.conditions) ab.conditions.forEach(c => {
                            if (c.condition_type !== undefined) activeConditions.add(`${conditionMap[c.condition_type] || c.condition_type}`);
                        });
                        if (ab.effects) ab.effects.forEach(e => {
                            if (e.effect_type !== undefined) activeEffects.add(`${effectMap[e.effect_type] || e.effect_type}`);
                        });
                        if (ab.costs) ab.costs.forEach(c => {
                            if (c.cost_type !== undefined) activeCosts.add(`${costMap[c.cost_type] || c.cost_type}`);
                        });
                    });
                }
            });
            // Live zone cards
            (p.live_zone || []).forEach(card => {
                if (card && card.abilities) {
                    card.abilities.forEach(ab => {
                        if (ab.trigger !== undefined) activeTriggers.add(`${triggerMap[ab.trigger] || ab.trigger}`);
                        if (ab.conditions) ab.conditions.forEach(c => {
                            if (c.condition_type !== undefined) activeConditions.add(`${conditionMap[c.condition_type] || c.condition_type}`);
                        });
                        if (ab.effects) ab.effects.forEach(e => {
                            if (e.effect_type !== undefined) activeEffects.add(`${effectMap[e.effect_type] || e.effect_type}`);
                        });
                        if (ab.costs) ab.costs.forEach(c => {
                            if (c.cost_type !== undefined) activeCosts.add(`${costMap[c.cost_type] || c.cost_type}`);
                        });
                    });
                }
            });
        });

        // Check if pending choice exists
        if (State.data.pending_choice) activeChoices.add('ACTIVE_PENDING_CHOICE');

        return {
            triggers: Array.from(activeTriggers).sort(),
            conditions: Array.from(activeConditions).sort(),
            effects: Array.from(activeEffects).sort(),
            costs: Array.from(activeCosts).sort(),
            choices: Array.from(activeChoices).sort()
        };
    },

    _renderActiveEnumsSection: () => {
        const enums = DebugModal._analyzeActiveEnums();
        if (!enums.triggers.length && !enums.conditions.length && !enums.effects.length && !enums.costs.length) {
            return '<div style="opacity:0.5; font-size:11px; padding:8px;">No active enums in current board state.</div>';
        }

        return `
            <div style="background: rgba(0,0,0,0.3); border-left: 3px solid var(--accent-green); padding: 12px; border-radius: 4px; margin-bottom: 16px;">
                <h3 style="margin-top:0; color:var(--accent-green); font-size:12px; text-transform:uppercase; letter-spacing:1px;">Active Enums (On Board Now)</h3>

                ${enums.triggers.length ? `
                <div style="margin-bottom: 10px;">
                    <div style="font-size: 10px; font-weight: bold; color: #2ecc71; text-transform: uppercase; margin-bottom: 4px;">Triggers:</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                        ${enums.triggers.map(t => `<span style="background: rgba(46,204,113,0.2); border: 1px solid rgba(46,204,113,0.5); color:#2ecc71; padding: 2px 6px; border-radius: 3px; font-size: 9px;">${t}</span>`).join('')}
                    </div>
                </div>
                ` : ''}

                ${enums.conditions.length ? `
                <div style="margin-bottom: 10px;">
                    <div style="font-size: 10px; font-weight: bold; color: #3498db; text-transform: uppercase; margin-bottom: 4px;">Conditions:</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                        ${enums.conditions.map(c => `<span style="background: rgba(52,152,219,0.2); border: 1px solid rgba(52,152,219,0.5); color:#3498db; padding: 2px 6px; border-radius: 3px; font-size: 9px;">${c}</span>`).join('')}
                    </div>
                </div>
                ` : ''}

                ${enums.effects.length ? `
                <div style="margin-bottom: 10px;">
                    <div style="font-size: 10px; font-weight: bold; color: #f39c12; text-transform: uppercase; margin-bottom: 4px;">Effects:</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                        ${enums.effects.map(e => `<span style="background: rgba(243,156,18,0.2); border: 1px solid rgba(243,156,18,0.5); color:#f39c12; padding: 2px 6px; border-radius: 3px; font-size: 9px;">${e}</span>`).join('')}
                    </div>
                </div>
                ` : ''}

                ${enums.costs.length ? `
                <div style="margin-bottom: 10px;">
                    <div style="font-size: 10px; font-weight: bold; color: #1abc9c; text-transform: uppercase; margin-bottom: 4px;">Cost Types:</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                        ${enums.costs.map(c => `<span style="background: rgba(26,188,156,0.2); border: 1px solid rgba(26,188,156,0.5); color:#1abc9c; padding: 2px 6px; border-radius: 3px; font-size: 9px;">${c}</span>`).join('')}
                    </div>
                </div>
                ` : ''}

                ${enums.choices.length ? `
                <div>
                    <div style="font-size: 10px; font-weight: bold; color: #e74c3c; text-transform: uppercase; margin-bottom: 4px;">Choice States:</div>
                    <div style="display: flex; flex-wrap: wrap; gap: 4px;">
                        ${enums.choices.map(c => `<span style="background: rgba(231,76,60,0.2); border: 1px solid rgba(231,76,60,0.5); color:#e74c3c; padding: 2px 6px; border-radius: 3px; font-size: 9px; font-weight: bold;">${c}</span>`).join('')}
                    </div>
                </div>
                ` : ''}
            </div>
        `;
    },

    renderBytecode: () => {
        const container = document.getElementById('debug-bytecode-content');
        if (!container || !State.data) return;
        const logs = State.data.bytecode_log || [];
        if (logs.length === 0) {
            container.innerHTML = '<div style="padding:40px; text-align:center; opacity:0.5;">No execution traces. Enable Debug Mode (<code>Network.toggleDebugMode()</code>) to capture bytecode execution logs.</div>';
            return;
        }
        container.innerHTML = logs.map(l => {
            let color = '#ccc';
            let bg = 'transparent';
            if (l.includes('ERR')) color = '#ff5555';
            if (l.includes('TRIGGER')) { color = '#fff'; bg = 'rgba(52, 152, 219, 0.2)'; }
            if (l.includes('EXECUTE')) { color = '#fff'; bg = 'rgba(46, 204, 113, 0.1)'; }
            return `<div style="padding: 2px 5px; border-bottom: 1px solid #222; color:${color}; background:${bg}; white-space: pre-wrap; word-break: break-all;">${l}</div>`;
        }).join('');
        container.scrollTop = container.scrollHeight;
    },

    renderJson: () => {
        const tx = document.getElementById('debug-json-textarea');
        if (!tx || !State.data) return;
        // Default to minimal board view (card IDs by zone only)
        DebugModal.renderMinimalJSON();
    },

    renderMinimalJSON: () => {
        if (!State.data) return;
        const getId = c => {
            if (!c) return -1;
            if (typeof c === 'object') return c.id ?? -1;
            return c;
        };
        const minimal = {
            phase: State.data.phase,
            turn: State.data.turn,
            players: State.data.players.map((p, pi) => ({
                _label: `Player ${pi + 1}`,
                stage: (p.stage || []).map(getId),
                live_zone: (p.live_zone || []).map(getId),
                hand: (p.hand || []).map(getId),
                success_lives: (p.success_lives || []).map(getId),
                energy: (p.energy || []).map(getId),
                discard: (p.discard || []).map(getId),
            }))
        };
        const tx = document.getElementById('debug-json-textarea');
        if (tx) tx.value = JSON.stringify(minimal, null, 2);
    },

    renderFullJSON: () => {
        const tx = document.getElementById('debug-json-textarea');
        if (tx && State.data) {
            tx.value = JSON.stringify(State.data, null, 2);
        }
    },

    applyCustomState: async () => {
        const tx = document.getElementById('debug-json-textarea');
        if (!tx) return;
        try {
            let newState = JSON.parse(tx.value);
            // Ensure any objects in the JSON are converted back to IDs
            newState = State.stripRichData(newState);
            const ok = await Network.applyState(JSON.stringify(newState));
            if (ok) {
                alert("Game Engine Overridden Successfully.");
                DebugModal.renderAll();
            } else {
                alert("Apply failed. Check server console for errors.");
            }
        } catch (e) {
            alert("Parse Error: " + e.message);
        }
    },

    applyBoardOnly: async () => {
        const tx = document.getElementById('debug-json-textarea');
        if (!tx || !State.data) return;
        try {
            let editState = JSON.parse(tx.value);
            // Ensure ID format for board override too
            editState = State.stripRichData(editState);
            const ok = await Network.boardOverride(JSON.stringify(editState));
            if (ok) {
                alert("Board Override Success.");
                DebugModal.renderAll();
            } else {
                alert("Override failed. Check server console.");
            }
        } catch (e) {
            alert("JSON Error: " + e.message);
        }
    },

    toggleEngineDebug: async () => {
        const ok = await Network.toggleDebugMode();
        if (ok) {
            alert("Engine Debug Mode Toggled.");
            DebugModal.renderAll();
        } else {
            alert("Toggle failed.");
        }
    },

    rewind: async () => {
        const ok = await Network.rewind();
        if (ok) {
            await DebugModal.renderAll();
            if (window.Rendering) Rendering.render();
        } else { alert("Undo failed (History empty?)"); }
    },

    redo: async () => {
        const ok = await Network.redo();
        if (ok) {
            await DebugModal.renderAll();
            if (window.Rendering) Rendering.render();
        } else { alert("Redo failed (Redo history empty?)"); }
    },

    copyStateString: () => {
        const textarea = document.getElementById('debug-string-textarea');
        if (textarea) {
            textarea.select();
            textarea.setSelectionRange(0, 99999); // For mobile devices
            try {
                document.execCommand('copy');
                alert('✓ State string copied to clipboard! Paste it in the textarea to restore this board state.');
            } catch (err) {
                // Fallback for some browsers
                navigator.clipboard.writeText(textarea.value).then(() => {
                    alert('✓ State string copied to clipboard! Paste it in the textarea to restore this board state.');
                });
            }
        } else if (State.data) {
            // Strip before copying from memory
            const source = State.stripRichData(State.data);
            const blob = btoa(unescape(encodeURIComponent(JSON.stringify(source))));
            navigator.clipboard.writeText(blob).then(() => alert("✓ State Sequence Copied to Clipboard."));
        }
    },

    loadStateString: async () => {
        const textarea = document.getElementById('debug-string-textarea');
        const blob = textarea ? textarea.value : prompt("Input State Sequence:");
        if (!blob) return;
        try {
            const json = JSON.parse(decodeURIComponent(escape(atob(blob))));
            // Ensure the state is clean before applying
            const cleanState = State.stripRichData(json);
            const ok = await Network.applyState(JSON.stringify(cleanState));
            if (ok) {
                alert("State Decoded and Applied. The undo/redo history will be preserved if the engines are compatible.");
                DebugModal.renderAll();
            } else { alert("Apply failed — check server console."); }
        } catch (e) { alert("Decode Error: " + e.message); }
    },

    saveState: () => {
        if (!State.data) return;
        // Strip rich data before saving to file
        const source = State.stripRichData(State.data);

        // Save both formats: JSON and Base64-encoded
        const jsonStr = JSON.stringify(source, null, 2);
        const base64Str = btoa(unescape(encodeURIComponent(JSON.stringify(source))));

        // Save JSON version
        const jsonBlob = new Blob([jsonStr], { type: 'application/json' });
        const jsonUrl = URL.createObjectURL(jsonBlob);
        const jsonLink = document.createElement('a');
        jsonLink.href = jsonUrl;
        jsonLink.download = `loveca_state_${Date.now()}.json`;
        document.body.appendChild(jsonLink);
        jsonLink.click();
        document.body.removeChild(jsonLink);
        URL.revokeObjectURL(jsonUrl);

        // Also save Base64 version for easy clipboard transfer
        const b64Blob = new Blob([base64Str], { type: 'text/plain' });
        const b64Url = URL.createObjectURL(b64Blob);
        const b64Link = document.createElement('a');
        b64Link.href = b64Url;
        b64Link.download = `loveca_state_${Date.now()}.b64`;
        document.body.appendChild(b64Link);
        b64Link.click();
        document.body.removeChild(b64Link);
        URL.revokeObjectURL(b64Url);

        alert(`State saved! 2 files created:\n- .json (readable)\n- .b64 (compact base64)`);
    },

    copyStandardizedJson: async () => {
        const data = await Network.fetchStandardizedState();
        if (data) {
            navigator.clipboard.writeText(JSON.stringify(data, null, 2))
                .then(() => alert("Standardized JSON copied to clipboard!"))
                .catch(err => alert("Failed to copy: " + err));
        } else {
            alert("Failed to fetch standardized state from server.");
        }
    },

    saveStandardizedJson: async () => {
        const data = await Network.fetchStandardizedState();
        if (data) {
            const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `lovecasim_master_${Date.now()}.json`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        } else {
            alert("Failed to fetch standardized state from server.");
        }
    },

    async runModelAnalysis() {
        const tx = document.getElementById('debug-json-textarea');
        const resultsDiv = document.getElementById('debug-analysis-results');
        if (!tx || !resultsDiv) return;

        try {
            const state = JSON.parse(tx.value);
            resultsDiv.style.display = 'block';
            resultsDiv.innerHTML = '<div style="color: #aaa;">Analyzing with Neural Network...</div>';

            const res = await fetch('/api/v1/analyze_model', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(state)
            });
            const data = await res.json();

            if (data.success) {
                let html = `
                    <div style="display: flex; gap: 10px; margin-bottom: 8px; font-weight: bold; border-bottom: 2px solid #444; padding-bottom: 4px;">
                        <span style="color: #2ecc71;">Win: ${(data.value.win_prob * 100).toFixed(1)}%</span>
                        <span style="color: #4a9eff;">Mom: ${data.value.momentum.toFixed(2)}</span>
                        <span style="color: #f1c40f;">Eff: ${(data.value.efficiency * 100).toFixed(1)}%</span>
                    </div>
                    <div style="font-family: monospace; display: flex; flex-direction: column; gap: 4px;">`;

                data.actions.forEach(a => {
                    const probColor = a.logit > 0 ? '#2ecc71' : (a.logit > -2 ? '#f1c40f' : '#aaa');
                    html += `
                        <div style="display: flex; justify-content: space-between; align-items: center; background: rgba(255,255,255,0.05); padding: 2px 6px; border-radius: 3px;">
                            <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-right: 10px;">${a.desc}</span>
                            <span style="color: ${probColor}; font-weight: bold; min-width: 45px; text-align: right;">${a.logit.toFixed(2)}</span>
                        </div>`;
                });
                html += '</div>';
                resultsDiv.innerHTML = html;
            } else {
                resultsDiv.innerHTML = `<div style="color: #e74c3c;">Error: ${data.error || 'Unknown error'}</div>`;
            }
        } catch (e) {
            resultsDiv.innerHTML = `<div style="color: #e74c3c;">Failed to analyze: ${e.message}</div>`;
        }
    }
};

// Wired to Globals
window.openDebugModal = DebugModal.openDebugModal;
window.closeDebugModal = DebugModal.closeDebugModal;
window.switchDebugTab = DebugModal.switchTab;
window.applyDebugState = DebugModal.applyCustomState;
window.copyDebugState = DebugModal.copyStateString;
window.loadDebugState = DebugModal.loadStateString;
window.renderFullJSON = DebugModal.renderFullJSON;
window.renderMinimalJSON = DebugModal.renderMinimalJSON;
window.DebugModal = DebugModal;
