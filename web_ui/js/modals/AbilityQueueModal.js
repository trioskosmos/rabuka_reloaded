import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOM_IDS } from '../constants_dom.js';
import * as i18n from '../i18n/index.js';

export const AbilityQueueModal = {
    open() {
        const state = State.data;
        // The engine serializes the full ability queue into
        // `ability_queue_entries` (every pending/active/completed entry), which is
        // what we want to render so the player can see ALL queued abilities (e.g.
        // a card with two [登場] debut abilities) from the start. The legacy
        // `ability_queue` field is never populated, so fall back to the entries.
        const entries = state?.ability_queue_entries || state?.ability_queue || [];
        const queue = entries.map((e, idx) => {
            const resolved = (e.card_id != null) ? State.resolveCardData(e.card_id) : null;
            return {
                source_card_id: e.card_id,
                source_name: resolved ? resolved.name : null,
                target_card_id: undefined,
                target_name: '',
                ability_text: e.ability_text || '',
                status: e.completed ? 'resolved' : 'pending',
                _idx: idx,
            };
        });
        const queueDepth = state?.queue_depth ?? 0;

        let html = '';
        if (queue.length === 0 && queueDepth === 0) {
            html = '<p style="opacity:0.5;text-align:center;padding:20px;">' + (i18n.t('no_queued_abilities') || 'No abilities in queue') + '</p>';
        } else {
            if (queueDepth > 0) {
                html += `<div style="margin-bottom:8px;font-size:0.8em;color:#ffcc00;">Queue Depth: ${queueDepth}</div>`;
            }
            const currentIdx = state?.ability_queue_current_index ?? -1;
            queue.forEach((entry, idx) => {
                const sourceCard = entry.source_card_id !== undefined ? State.resolveCardData(entry.source_card_id) : null;
                const sourceName = sourceCard ? sourceCard.name : (entry.source_name || 'Unknown');
                const targetCard = entry.target_card_id !== undefined ? State.resolveCardData(entry.target_card_id) : null;
                const targetName = targetCard ? targetCard.name : (entry.target_name || '');
                const abilityText = entry.ability_text || '';
                const status = entry.status || 'pending';
                const statusColor = status === 'resolved' ? '#2ecc71' : status === 'triggered' ? '#f39c12' : '#3498db';
                const isCurrent = idx === currentIdx;
                const activeOutline = isCurrent ? 'box-shadow:0 0 0 2px #ffcc00;' : '';

                html += `<div class="ability-queue-entry" style="padding:8px 10px;margin-bottom:6px;border-radius:6px;background:rgba(255,255,255,0.04);border-left:3px solid ${statusColor};${activeOutline}">`;
                if (isCurrent) {
                    html += `<div style="font-size:0.62em;color:#ffcc00;margin-bottom:2px;text-transform:uppercase;">▶ ${i18n.t('active') || 'Active'}</div>`;
                }
                html += `<div style="display:flex;justify-content:space-between;align-items:center;">`;
                html += `<strong style="color:#cc88ff;font-size:0.85rem;">${sourceName}</strong>`;
                html += `<span style="font-size:0.7em;color:${statusColor};text-transform:uppercase;">${status}</span>`;
                html += `</div>`;
                if (targetName) html += `<div style="font-size:0.75em;opacity:0.7;margin-top:2px;">→ ${targetName}</div>`;
                if (abilityText) {
                    const enriched = Tooltips.enrichAbilityText ? Tooltips.enrichAbilityText(abilityText) : abilityText;
                    html += `<div style="font-size:0.75em;opacity:0.6;margin-top:4px;line-height:1.3;">${enriched}</div>`;
                }
                html += `</div>`;
            });
        }

        const isMobile = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
        if (isMobile) {
            const selModal = document.getElementById(DOM_IDS.SELECTION_MODAL);
            const selContent = document.getElementById(DOM_IDS.SELECTION_CONTENT);
            const selTitle = document.getElementById('selection-title');
            const viewBtn = document.getElementById('selection-view-card-btn');
            const selectBtn = document.getElementById('selection-select-btn');
            if (viewBtn) viewBtn.style.display = 'none';
            if (selectBtn) selectBtn.style.display = 'none';
            const closeBtn = document.getElementById('selection-close-btn');
            if (closeBtn) closeBtn.style.display = '';
            if (selTitle) selTitle.textContent = i18n.t('ability_queue') || 'Ability Queue';
            if (selContent) {
                selContent.innerHTML = html;
                selContent.style.overflowY = 'auto';
            }
            ModalManager.show(DOM_IDS.SELECTION_MODAL);
        } else {
            const body = document.getElementById('ability-queue-body');
            if (body) body.innerHTML = html;
            ModalManager.show(DOM_IDS.MODAL_ABILITY_QUEUE);
        }
    },

    close() {
        const isMobile = typeof window.__isMobile === 'function' ? window.__isMobile() : false;
        if (isMobile && ModalManager.isVisible(DOM_IDS.SELECTION_MODAL)) {
            ModalManager.hide(DOM_IDS.SELECTION_MODAL);
            const viewBtn = document.getElementById('selection-view-card-btn');
            const selectBtn = document.getElementById('selection-select-btn');
            const closeBtn = document.getElementById('selection-close-btn');
            if (viewBtn) viewBtn.style.display = '';
            if (selectBtn) selectBtn.style.display = '';
            if (closeBtn) closeBtn.style.display = 'none';
        } else {
            ModalManager.hide(DOM_IDS.MODAL_ABILITY_QUEUE);
        }
    }
};
