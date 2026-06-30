import { State } from '../state.js';
import { ModalManager } from '../utils/ModalManager.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';
import { ActionButtons } from '../components/ActionButtons.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

// Navigation state
let _handCardDatas = [];
let _handActions = [];   // per-card action lists
let _currentIdx = 0;

function closeAndDoAction(action) {
    PlayActionModal.close();
    if (window.doAction) window.doAction(action);
}

function buildHandActions(state) {
    // Build a map of hand card_index → list of play_member_to_stage actions
    const legal = state?.legal_actions || [];
    const pp = State.perspectivePlayer;
    const p = pp === 0 ? state?.player1 : state?.player2;
    const hand = p?.hand?.cards || [];
    const map = {};
    legal.forEach(a => {
        if (a.action_type !== 'play_member_to_stage') return;
        const ci = a.parameters?.card_index;
        if (ci !== undefined && hand[ci]) {
            if (!map[ci]) map[ci] = [];
            map[ci].push(a);
        }
    });
    return map;
}

function renderHandCard(index) {
    const modal = document.getElementById(DOM_IDS.MODAL_PLAY_ACTION);
    if (!modal) return;
    const cardPreview = document.getElementById('play-action-card-preview');
    const cardNameEl = document.getElementById('play-action-card-name');
    const actionsContainer = document.getElementById('play-action-buttons');

    const cardData = _handCardDatas[index];
    const actions = _handActions[index];
    if (!cardData || !actions || actions.length === 0) return;

    const first = actions[0];
    const params = first.parameters || {};
    const cardNo = params.card_no || cardData.card_no;
    const cardName = params.card_name || cardData.name || 'Unknown';

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
        const translated = (cardNo && State.resolveCardData(cardNo)) ? (window.translateCard ? window.translateCard(State.resolveCardData(cardNo)) : null) : null;
        const displayName = (translated && translated.name) ? translated.name : cardName;
        const energyIcon = '<img src="img/texticon/icon_energy.png" style="height:14px;vertical-align:middle;">';
        const baseCost = params.base_cost !== undefined ? params.base_cost : (params.cost || '?');
        const posStr = `[${index + 1}/${_handCardDatas.length}]`;
        cardNameEl.innerHTML = `${displayName} ${posStr} ${energyIcon}${baseCost}`;
    }

    // Nav buttons
    const prevBtn = document.getElementById('play-nav-prev');
    const nextBtn = document.getElementById('play-nav-next');
    if (prevBtn) prevBtn.style.visibility = index > 0 ? 'visible' : 'hidden';
    if (nextBtn) nextBtn.style.visibility = index < _handCardDatas.length - 1 ? 'visible' : 'hidden';

    if (actionsContainer) {
        actionsContainer.innerHTML = '';
        const availableAreas = params.available_areas;
        const doubleBatonPairs = params.double_baton_pairs;
        const hasDoubleBaton = doubleBatonPairs && doubleBatonPairs.length > 0;

        const areaLabels = { 'left': i18n.t('area_left'), 'center': i18n.t('area_center'), 'right': i18n.t('area_right') };
        const areaOrder = ['left', 'center', 'right'];

        if (availableAreas && availableAreas.some(a => a.available)) {
            const areasDiv = document.createElement('div');
            areasDiv.className = 'action-group-buttons grid-3';
            areaOrder.forEach(expectedArea => {
                const areaInfo = availableAreas.find(a => a.area === expectedArea);
                if (areaInfo && areaInfo.available) {
                    const label = areaLabels[areaInfo.area] || areaInfo.area;
                    const cost = areaInfo.cost;
                    const isBaton = areaInfo.is_baton_touch;
                    const areaActionCopy = { ...first };
                    areaActionCopy.parameters = { ...first.parameters, stage_area: areaInfo.area };
                    const btn = ActionButtons.createActionButton(areaActionCopy, true, '', State.data);
                    const costText = isBaton ? `${label} (${cost} - Baton)` : `${label} (${cost})`;
                    btn.innerHTML = `<span>${costText}</span>`;
                    btn.style.width = '100%';
                    btn.style.padding = '12px 8px';
                    btn.style.fontSize = '0.95rem';
                    btn.onclick = (e) => { e.stopPropagation(); closeAndDoAction(areaActionCopy); };
                    areasDiv.appendChild(btn);
                } else {
                    const spacer = document.createElement('div');
                    spacer.style.visibility = 'hidden';
                    spacer.style.minHeight = '44px';
                    areasDiv.appendChild(spacer);
                }
            });
            actionsContainer.appendChild(areasDiv);
        }

        if (hasDoubleBaton) {
            const dbDiv = document.createElement('div');
            dbDiv.style.cssText = 'margin-top:10px;border-top:1px dashed rgba(255,215,0,0.3);background:rgba(0,0,0,0.15);padding:8px;border-radius:4px;';
            const dbLabel = document.createElement('div');
            dbLabel.style.cssText = 'font-size:0.75em;color:#ffda79;margin-bottom:6px;font-weight:bold;';
            dbLabel.textContent = i18n.t('double_baton') || 'DOUBLE BATON';
            dbDiv.appendChild(dbLabel);
            const areaIndexMap = { 'left': 0, 'center': 1, 'right': 2 };
            const pairGroups = {};
            doubleBatonPairs.forEach(pair => {
                const key = pair.areas.sort().join('&');
                if (!pairGroups[key]) pairGroups[key] = [];
                pairGroups[key].push(pair);
            });
            Object.keys(pairGroups).forEach(key => {
                const row = document.createElement('div');
                row.className = 'action-group-buttons grid-3';
                row.style.cssText = 'padding:2px;border-radius:4px;margin-top:2px;';
                const areas = key.split('&');
                areaOrder.forEach(expectedArea => {
                    const pairForPlacement = pairGroups[key].find(p => p.placement === expectedArea);
                    if (pairForPlacement) {
                        const labelA = areaLabels[areas[0]] || areas[0];
                        const labelB = areaLabels[areas[1]] || areas[1];
                        const placeLabel = areaLabels[expectedArea] || expectedArea;
                        const replaceIndices = areas.map(a => areaIndexMap[a]);
                        const dbActionParams = {
                            card_id: params.card_id,
                            card_index: params.card_index,
                            card_indices: replaceIndices,
                            stage_area: expectedArea,
                            use_baton_touch: true,
                            card_name: params.card_name,
                            card_no: params.card_no,
                        };
                        const btn = ActionButtons.createActionButton(
                            { action_type: 'play_member_to_stage', parameters: dbActionParams },
                            true, '', State.data
                        );
                        const costText = `${labelA}&${labelB} → ${placeLabel} (${pairForPlacement.cost} - Double)`;
                        btn.innerHTML = `<span>${costText}</span>`;
                        btn.style.width = '100%';
                        btn.onclick = (e) => { e.stopPropagation(); closeAndDoAction({ action_type: 'play_member_to_stage', parameters: dbActionParams }); };
                        row.appendChild(btn);
                    } else {
                        const spacer = document.createElement('div');
                        spacer.style.cssText = 'min-height:36px;display:flex;align-items:center;justify-content:center;opacity:0.2;font-size:0.6em;border:1px dashed rgba(255,255,255,0.1);';
                        spacer.textContent = '--';
                        row.appendChild(spacer);
                    }
                });
                dbDiv.appendChild(row);
            });
            actionsContainer.appendChild(dbDiv);
        }

        if (actionsContainer.children.length === 0) {
            actionsContainer.innerHTML = '<p style="opacity:0.6;text-align:center;padding:16px;">No playable areas available</p>';
        }
    }
}

export const PlayActionModal = {
    open(cardData, actions) {
        if (!actions || actions.length === 0) return;

        const modal = document.getElementById(DOM_IDS.MODAL_PLAY_ACTION);
        if (!modal) return;

        // Build hand card index for navigation
        const state = State.data;
        const pp = State.perspectivePlayer;
        const p = pp === 0 ? state?.player1 : state?.player2;
        const hand = p?.hand?.cards || [];
        const handMap = buildHandActions(state);
        _handCardDatas = [];
        _handActions = [];
        const sortedIndices = Object.keys(handMap).map(Number).sort((a, b) => a - b);
        sortedIndices.forEach(ci => {
            if (hand[ci]) {
                _handCardDatas.push(hand[ci]);
                _handActions.push(handMap[ci]);
            }
        });

        // Find the index of the tapped card
        const tapIdx = actions[0]?.parameters?.card_index;
        _currentIdx = _handCardDatas.length > 0 ? 0 : 0;
        if (tapIdx !== undefined) {
            const found = sortedIndices.indexOf(tapIdx);
            if (found >= 0) _currentIdx = found;
        }

        ModalManager.show(DOM_IDS.MODAL_PLAY_ACTION);
        renderHandCard(_currentIdx);
    },

    navigatePrev() {
        if (_currentIdx > 0) { _currentIdx--; renderHandCard(_currentIdx); }
    },

    navigateNext() {
        if (_currentIdx < _handCardDatas.length - 1) { _currentIdx++; renderHandCard(_currentIdx); }
    },

    close() {
        ModalManager.hide(DOM_IDS.MODAL_PLAY_ACTION);
    }
};
