import { State } from '../state.js';
import * as i18n from '../i18n/index.js';
import { Tooltips } from '../ui_tooltips.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';

const HEART_ICONS = ['heart00', 'heart01', 'heart02', 'heart03', 'heart04', 'heart05', 'heart06', 'all'];

function hIcon(index) {
    if (index === 7 || isNaN(index) || index >= HEART_ICONS.length) {
        return '<img src="img/texticon/icon_all.png" class="heart-mini-icon" alt="">';
    }
    return `<img src="img/texticon/heart_0${index}.png" class="heart-mini-icon" alt="">`;
}

function formatLogDetailBody(body) {
    let enriched = Tooltips.enrichAbilityText(body || '');
    enriched = enriched.replace(/P1 /g, '<span class="log-p-badge p1">P1</span> ');
    enriched = enriched.replace(/P2 /g, '<span class="log-p-badge p2">P2</span> ');
    enriched = enriched.replace(/\[Turn (\d+)\]/g, '<span class="log-turn-prefix">[Turn $1]</span>');
    return enriched;
}

export const LogDetailModal = {
    init: () => {
        const modal = DOMUtils.getElement(DOM_IDS.MODAL_LOG_DETAIL);
        if (modal) {
            ModalManager.setupBackdropClose(DOM_IDS.MODAL_LOG_DETAIL, LogDetailModal.close);
        }
    },

    open: (entryType, body, groupId) => {
        const shown = ModalManager.show(DOM_IDS.MODAL_LOG_DETAIL);
        if (!shown) return;

        const titleEl = DOMUtils.getElement(DOM_IDS.LOG_DETAIL_TITLE);
        const contentEl = DOMUtils.getElement(DOM_IDS.LOG_DETAIL_CONTENT);
        if (!contentEl) return;

        const typeLabel = LogDetailModal._getTypeLabel(entryType);
        if (titleEl) titleEl.textContent = typeLabel;

        contentEl.innerHTML = '';

        const wrapper = document.createElement('div');
        wrapper.className = 'log-detail-wrapper';

        const logLine = document.createElement('div');
        logLine.className = 'log-detail-original';
        logLine.innerHTML = formatLogDetailBody(body);
        wrapper.appendChild(logLine);

        const expanded = document.createElement('div');
        expanded.className = 'log-detail-expanded';

        expanded.appendChild(LogDetailModal._buildContextCards(body));
        expanded.appendChild(LogDetailModal._buildEffectBreakdown(body));

        wrapper.appendChild(expanded);
        contentEl.appendChild(wrapper);
    },

    close: () => {
        ModalManager.hide(DOM_IDS.MODAL_LOG_DETAIL);
    },

    _getTypeLabel: (entryType) => {
        const labels = {
            'score': i18n.t('event_score'),
            'effect': i18n.t('event_effect'),
            'heart_effect': i18n.t('event_heart'),
            'ability_effect': i18n.t('event_effect'),
            'performance': i18n.t('event_performance'),
            'action': i18n.t('event_play'),
            'phase': i18n.t('event_phase'),
            'generic': i18n.t('rule_log'),
        };
        return labels[entryType] || i18n.t('rule_log');
    },

    _buildContextCards: (body) => {
        const container = document.createElement('div');
        container.className = 'log-detail-cards-row';

        const cardNames = LogDetailModal._extractCardNames(body);
        cardNames.forEach(name => {
            const cardData = State.resolveCardDataByName(name);
            if (cardData && cardData.card_no) {
                const imgPath = resolveCardImagePath(cardData.card_no);
                if (imgPath) {
                    const img = document.createElement('img');
                    img.src = imgPath;
                    img.className = 'log-detail-card-thumb';
                    img.alt = cardData.name || name;
                    container.appendChild(img);
                }
            }
        });

        return container;
    },

    _buildEffectBreakdown: (body) => {
        const div = document.createElement('div');
        div.className = 'log-detail-breakdown';

        const heartMatch = body.match(/(ハート|heart)\s*([\+\-]\d+)\s*\(?(\w+)\)?/i);
        const bladeMatch = body.match(/(ブレード|blade)\s*([\+\-]\d+)/i);
        const scoreMatch = body.match(/(スコア|score).*?(\d+)/i);

        if (heartMatch || bladeMatch || scoreMatch) {
            const list = document.createElement('ul');
            list.className = 'log-detail-effects-list';

            if (heartMatch) {
                const li = document.createElement('li');
                const colorIdx = parseInt(heartMatch[3]?.replace('heart', '') || '0');
                li.innerHTML = `${i18n.t('heart')} ${heartMatch[2]} ${hIcon(colorIdx)}`;
                list.appendChild(li);
            }
            if (bladeMatch) {
                const li = document.createElement('li');
                li.innerHTML = `${i18n.t('blade')} ${bladeMatch[2]} <img src="img/texticon/icon_blade.png" class="heart-mini-icon" alt="">`;
                list.appendChild(li);
            }
            if (scoreMatch) {
                const li = document.createElement('li');
                li.innerHTML = `${i18n.t('score')} ${scoreMatch[2]} <img src="img/texticon/icon_score.png" class="heart-mini-icon" alt="">`;
                list.appendChild(li);
            }

            div.appendChild(list);
        }

        if (body.includes('PASS') || body.includes('FAIL')) {
            const isPass = body.includes('PASS') && !body.includes('FAIL');
            const result = document.createElement('div');
            result.className = `log-detail-result ${isPass ? 'success' : 'failure'}`;

            const passMatch = body.match(/Score:\s*(\d+)/);
            const score = passMatch ? passMatch[1] : '?';
            result.innerHTML = `<strong>${isPass ? '✓ PASS' : '✗ FAIL'}</strong> ${i18n.t('score')}: ${score}`;
            div.appendChild(result);
        }

        return div;
    },

    _extractCardNames: (body) => {
        if (!body) return [];
        const names = [];
        const cardMatch = body.match(/"([^"]+)"/g);
        if (cardMatch) {
            cardMatch.forEach(m => names.push(m.replace(/"/g, '')));
        }
        return names;
    },
};