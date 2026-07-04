import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { Tooltips } from '../ui_tooltips.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';
import { TextEnricher } from '../utils/TextEnricher.js';

export const StageAbilityModal = {
    open(cardData, abilityActions) {
        if (!cardData) return;

        const modal = document.getElementById(DOM_IDS.MODAL_STAGE_ABILITY);
        if (!modal) return;

        const cardPreview = document.getElementById('stage-ability-card-preview');
        const cardNameEl = document.getElementById('stage-ability-card-name');
        const abilityContainer = document.getElementById('stage-ability-list');

        const cardNo = cardData.card_no;
        const cardName = cardData.name || 'Unknown Card';

        if (cardPreview) {
            cardPreview.innerHTML = '';
            if (cardNo && cardNo !== '-1' && cardNo !== -1) {
                const imgPath = resolveCardImagePath(cardNo);
                if (imgPath) {
                    const img = document.createElement('img');
                    img.src = fixImg(imgPath);
                    img.alt = cardName;
                    img.style.maxWidth = '100%';
                    img.style.height = 'auto';
                    img.style.borderRadius = '6px';
                    cardPreview.appendChild(img);
                }
            }
        }

        if (cardNameEl) {
            const translated = window.translateCard ? window.translateCard(cardData) : null;
            const displayName = (translated && translated.name) ? translated.name : cardName;
            cardNameEl.textContent = displayName;
        }

        // Render full ability text
        const textContainer = document.getElementById('stage-ability-card-text');
        if (textContainer) {
            textContainer.innerHTML = '';
            const rawText = cardData.ability_text || cardData.text || cardData.original_text || '';
            if (rawText) {
                textContainer.innerHTML = TextEnricher.enrichAbilityText(rawText);
            }
        }

        if (abilityContainer) {
            abilityContainer.innerHTML = '';

            if (abilityActions && abilityActions.length > 0) {
                abilityActions.forEach(a => {
                    const btn = document.createElement('button');
                    btn.className = 'btn action-btn';
                    btn.style.cssText = 'width:100%;padding:10px 12px;margin-bottom:6px;text-align:left;display:flex;flex-direction:column;align-items:flex-start;gap:2px;border-left:3px solid #9966ff;';

                    const params = a.parameters || {};
                    const actionText = Tooltips.getEffectiveActionText ? Tooltips.getEffectiveActionText(a) : (params.description || a.description || i18n.t('act_ability'));

                    const nameSpan = document.createElement('span');
                    nameSpan.style.fontWeight = 'bold';
                    nameSpan.style.color = '#cc88ff';
                    const cleanText = actionText.replace(/<[^>]+>/g, '').trim();
                    nameSpan.textContent = cleanText || i18n.t('act_ability');
                    btn.appendChild(nameSpan);

                    if (params.cost !== undefined) {
                        const costSpan = document.createElement('span');
                        costSpan.style.cssText = 'font-size:0.75em;opacity:0.7;';
                        const energyIcon = '<img src="img/texticon/icon_energy.png" style="height:12px;vertical-align:middle;">';
                        costSpan.innerHTML = `${energyIcon} ${params.cost}`;
                        btn.appendChild(costSpan);
                    }

                    btn.onclick = (e) => {
                        e.stopPropagation();
                        StageAbilityModal.close();
                        if (window.doAction) window.doAction(a);
                    };
                    abilityContainer.appendChild(btn);
                });
            } else {
                abilityContainer.innerHTML = '<p style="opacity:0.5;text-align:center;padding:16px;">No abilities can be activated</p>';
            }
        }

        ModalManager.show(DOM_IDS.MODAL_STAGE_ABILITY);
    },

    close() {
        ModalManager.hide(DOM_IDS.MODAL_STAGE_ABILITY);
    }
};
