import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOM_IDS } from '../constants_dom.js';
import * as i18n from '../i18n/index.js';

export const AbilityQueueModal = {
    open() {
        const modal = document.getElementById(DOM_IDS.MODAL_ABILITY_QUEUE);
        if (!modal) return;

        const body = document.getElementById('ability-queue-body');
        if (!body) { ModalManager.show(DOM_IDS.MODAL_ABILITY_QUEUE); return; }

        const state = State.data;
        const queue = state?.ability_queue || [];
        const queueDepth = state?.queue_depth ?? 0;

        let html = '';
        if (queue.length === 0 && queueDepth === 0) {
            html = '<p style="opacity:0.5;text-align:center;padding:20px;">' + (i18n.t('no_queued_abilities') || 'No abilities in queue') + '</p>';
        } else {
            if (queueDepth > 0) {
                html += `<div style="margin-bottom:8px;font-size:0.8em;color:#ffcc00;">Queue Depth: ${queueDepth}</div>`;
            }
            queue.forEach((entry, idx) => {
                const sourceCard = entry.source_card_id !== undefined ? State.resolveCardData(entry.source_card_id) : null;
                const sourceName = sourceCard ? sourceCard.name : (entry.source_name || 'Unknown');
                const targetCard = entry.target_card_id !== undefined ? State.resolveCardData(entry.target_card_id) : null;
                const targetName = targetCard ? targetCard.name : (entry.target_name || '');
                const abilityText = entry.ability_text || '';
                const status = entry.status || 'pending';
                const statusColor = status === 'resolved' ? '#2ecc71' : status === 'triggered' ? '#f39c12' : '#3498db';

                html += `<div class="ability-queue-entry" style="padding:8px 10px;margin-bottom:6px;border-radius:6px;background:rgba(255,255,255,0.04);border-left:3px solid ${statusColor};">`;
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
        body.innerHTML = html;

        ModalManager.show(DOM_IDS.MODAL_ABILITY_QUEUE);
    },

    close() {
        ModalManager.hide(DOM_IDS.MODAL_ABILITY_QUEUE);
    }
};
