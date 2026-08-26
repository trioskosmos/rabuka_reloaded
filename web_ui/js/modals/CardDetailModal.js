import { State } from '../state.js';
import { TextEnricher } from '../utils/TextEnricher.js';
import { ModalManager } from '../utils/ModalManager.js';
import { renderActiveEffects, renderRecentApplications } from '../utils/Attribution.js';
import { resolveCardImagePath } from '../components/CardRenderer.js';
import { ActionButtons } from '../components/ActionButtons.js';
import { DOM_IDS } from '../constants_dom.js';
import { fixImg } from '../constants.js';
import * as i18n from '../i18n/index.js';

// Navigation state
let navCards = [];
let navIndex = 0;
let navCurrentZone = '';
let _selectCallback = null;

function buildAllCards() {
    const state = State.data;
    if (!state) return {};
    const pp = State.perspectivePlayer;
    const p = pp === 0 ? state.player1 : state.player2;
    const opp = pp === 0 ? state.player2 : state.player1;
    if (!p) return {};

    const resolve = (c) => {
        if (!c) return null;
        if (typeof c === 'number') return State.resolveCardData(c);
        if (c.card_no && !c.name) {
            const resolved = State.resolveCardData(c.card_no);
            if (resolved) return resolved;
        }
        return c;
    };

    const isValid = (c) => {
        if (!c) return false;
        const card = resolve(c);
        return card && card.card_no && card.card_no !== -1 && card.card_no !== -2 && card.card_no !== '-1' && card.card_no !== '-2';
    };

    const cards = {};

    const handArr = Array.isArray(p.hand?.cards) ? p.hand.cards : [];
    cards['hand'] = handArr.map(resolve).filter(isValid);

    const stageObj = p.stage || {};
    const stageSlots = ['left_side', 'center', 'right_side'];
    cards['stage'] = stageSlots.map(s => resolve(stageObj[s])).filter(isValid);

    const liveArr = Array.isArray(p.live_zone?.cards) ? p.live_zone.cards : [];
    cards['live'] = liveArr.map(resolve).filter(isValid);

    const discArr = Array.isArray(p.discard?.cards) ? p.discard.cards : [];
    cards['discard'] = discArr.map(resolve).filter(isValid);

    const under = [];
    stageSlots.forEach(slot => {
        const uArr = Array.isArray(stageObj[slot + '_under']) ? stageObj[slot + '_under'] : [];
        uArr.forEach(c => { const r = resolve(c); if (isValid(r)) under.push(r); });
    });
    if (under.length) cards['under'] = under;

    const oppStageObj = opp?.stage || {};
    const oppStage = stageSlots.map(s => resolve(oppStageObj[s])).filter(isValid);
    if (oppStage.length) cards['opp_stage'] = oppStage;

    return cards;
}

function render() {
    const titleEl = document.getElementById('card-detail-title');
    const imageEl = document.getElementById('card-detail-image');
    const textEl = document.getElementById('card-detail-text');
    const statsEl = document.getElementById('card-detail-stats');
    const actionsEl = document.getElementById('card-detail-actions');
    const footerEl = document.getElementById('card-detail-position');

    const card = navCards[navIndex];
    if (!card) return;

    const cardNo = card.card_no;
    const resolved = (card.card_no && card.card_no !== -1 && card.card_no !== -2 && card.card_no !== '-1' && card.card_no !== '-2') ? State.resolveCardData(card.card_no) : null;
    const cardObj = resolved || card;

    const isHidden = card.hidden || card.is_hidden || card.card_no === -1 || card.card_no === -2 || card.card_no === '-1' || card.card_no === '-2' || card.card_no <= 0;
    const translated = window.translateCard ? window.translateCard(cardObj) : null;

    // Title
    if (titleEl) {
        let t = translated?.name || cardObj.name || 'Card';
        if (cardNo && cardNo !== -1 && cardNo !== -2 && cardNo !== '-1' && cardNo !== '-2') {
            t += ` <span style="opacity:0.5;font-size:0.75em;font-family:monospace;">${cardNo}</span>`;
        }
        titleEl.innerHTML = t;
    }

    // Image
    if (imageEl) {
        imageEl.innerHTML = '';
        if (cardNo && cardNo !== -1 && cardNo !== -2 && cardNo !== '-1' && cardNo !== '-2') {
            const imgPath = resolveCardImagePath(cardNo);
            if (imgPath) {
                const img = document.createElement('img');
                img.src = fixImg(imgPath);
                img.alt = cardObj.name || '';
                imageEl.appendChild(img);
            }
        }
    }

    // Stats
    if (statsEl) {
        statsEl.innerHTML = '';
        if (!isHidden) {
            const b = [];
            if (cardObj.energy_cost !== undefined) b.push(`<span class="stat-badge">Cost: ${cardObj.energy_cost}</span>`);
            if (cardObj.power !== undefined) b.push(`<span class="stat-badge">Power: ${cardObj.power}</span>`);
            if (cardObj.soul !== undefined) b.push(`<span class="stat-badge">Soul: ${cardObj.soul}</span>`);
            if (b.length) statsEl.innerHTML = b.join(' ');
        }
    }

    // Text
    if (textEl) {
        textEl.innerHTML = '';
        if (isHidden) {
            textEl.innerHTML = '<p style="opacity:0.5;">Card is hidden</p>';
        } else {
            let html = '';
            const groups = (translated?.groups || cardObj.groups || []).join(', ');
            const units = (translated?.units || cardObj.units || []).join(', ');
            const parts = [];
            if (groups) parts.push(`Groups: ${groups}`);
            if (units) parts.push(`Units: ${units}`);
            if (parts.length) html += `<div style="font-size:0.75em;opacity:0.7;margin-bottom:4px;">${parts.join(' | ')}</div>`;

            const rawText = TextEnricher.getEffectiveRawText(cardObj) || '';
            if (rawText) html += `<div class="card-detail-ability">${TextEnricher.enrichAbilityText(rawText)}</div>`;
            textEl.innerHTML = html;
        }
    }

    // Active effects / bonus attribution — shows WHERE each bonus on this
    // card comes from (source card + ability text). Same modal serves
    // desktop and mobile.
    const actionsElRef = document.getElementById('card-detail-actions');
    let attrHost = document.getElementById('card-detail-attribution');
    if (!attrHost && actionsElRef?.parentNode) {
        attrHost = document.createElement('div');
        attrHost.id = 'card-detail-attribution';
        actionsElRef.parentNode.insertBefore(attrHost, actionsElRef);
    }
    if (attrHost) {
        attrHost.innerHTML = '';
        if (!isHidden && typeof card.id === 'number' && card.id >= 0) {
            attrHost.appendChild(renderActiveEffects(card.id));
            const recent = renderRecentApplications(card.id);
            if (recent) attrHost.appendChild(recent);
        }
    }

    // Legal actions
    if (actionsEl) {
        actionsEl.innerHTML = '';
        const legal = State.data?.legal_actions || [];
        if (legal.length > 0 && !isHidden && State.uiMode === 'view') {
            const cardActions = legal.filter(a => {
                const params = a.parameters || {};
                if (params.card_id !== undefined && params.card_id === card.id) return true;
                if (params.card_no !== undefined && String(params.card_no) === String(cardNo)) return true;
                if (params.card_index !== undefined && navCurrentZone === 'hand') {
                    const idx = navCards.indexOf(card);
                    if (params.card_index === idx) return true;
                }
                if (params.card_indices && Array.isArray(params.card_indices) && navCurrentZone === 'hand') {
                    const idx = navCards.indexOf(card);
                    if (params.card_indices.includes(idx)) return true;
                }
                return false;
            });
            if (cardActions.length > 0) {
                const label = document.createElement('div');
                label.className = 'card-detail-actions-label';
                label.textContent = i18n.t('legal_actions') || 'Legal Actions';
                actionsEl.appendChild(label);

                // Group play_member_to_stage actions by card_no (same as ActionListView)
                const playActionsByCard = {};
                const otherActions = [];
                cardActions.forEach(a => {
                    if (a.action_type === 'play_member_to_stage' && a.parameters?.card_no !== undefined) {
                        const cn = a.parameters.card_no;
                        if (!playActionsByCard[cn]) playActionsByCard[cn] = [];
                        playActionsByCard[cn].push(a);
                    } else {
                        otherActions.push(a);
                    }
                });

                // Render play actions per card using ActionButtons (same pattern as ActionListView)
                Object.keys(playActionsByCard).forEach(cn => {
                    const actions = playActionsByCard[cn];
                    const firstA = actions[0];
                    const groupDiv = document.createElement('div');
                    groupDiv.className = 'card-detail-play-group';

                    const header = document.createElement('div');
                    header.className = 'card-detail-play-header';
                    const energyIcon = '<img src="img/texticon/icon_energy.png" style="height:12px;vertical-align:middle;">';
                    const displayCost = firstA.parameters?.base_cost ?? 0;
                    let cleanName = firstA.parameters?.card_name ?? firstA.description ?? 'Unknown';
                    if (State.currentLang === 'en' && firstA.parameters?.card_id !== undefined) {
                        const sourceCard = State.resolveCardData(firstA.parameters.card_id);
                        if (sourceCard) cleanName = (window.translateCard ? window.translateCard(sourceCard).name : sourceCard.name) || cleanName;
                    }
                    header.innerHTML = `<span class="truncate-name">${cleanName}</span> <span style="opacity:0.7;font-size:0.8em;">${energyIcon}${displayCost}</span>`;
                    groupDiv.appendChild(header);

                    const availableAreas = firstA.parameters?.available_areas;
                    if (availableAreas && availableAreas.some(a => a.available)) {
                        const areasDiv = document.createElement('div');
                        areasDiv.className = 'action-group-buttons';
                        const areaLabels = { left: i18n.t('area_left') || 'Left', center: i18n.t('area_center') || 'Center', right: i18n.t('area_right') || 'Right' };
                        const areaOrder = ['left', 'center', 'right'];
                        areaOrder.forEach(areaName => {
                            const areaInfo = availableAreas.find(a => a.area === areaName);
                            if (areaInfo && areaInfo.available) {
                                const areaAction = { ...firstA, parameters: { ...firstA.parameters, stage_area: areaName } };
                                const btn = ActionButtons.createActionButton(areaAction, true, '', State.data);
                                const costText = areaInfo.is_baton_touch ? `${areaLabels[areaName]} ${areaInfo.cost} Baton` : `${areaLabels[areaName]} ${areaInfo.cost}`;
                                btn.innerHTML = `<span>${costText}</span>`;
                                btn.style.width = '100%';
                                btn.dataset.zoneArea = areaName;
                                btn.onclick = (e) => { e.stopPropagation(); CardDetailModal.close(); if (window.doAction) window.doAction(areaAction); };
                                areasDiv.appendChild(btn);
                            } else {
                                const spacer = document.createElement('div');
                                spacer.style.cssText = 'flex:1;min-height:36px;border:1px solid transparent;border-right:none;';
                                areasDiv.appendChild(spacer);
                            }
                        });
                        groupDiv.appendChild(areasDiv);
                    }

                    const doubleBatonPairs = firstA.parameters?.double_baton_pairs;
                    const hasDoubleBaton = doubleBatonPairs && doubleBatonPairs.length > 0;
                    if (hasDoubleBaton) {
                        const areaIndexMap = { left: 0, center: 1, right: 2 };
                        const areaOrder = ['left', 'center', 'right'];
                        const areaLabels = { left: i18n.t('area_left'), center: i18n.t('area_center'), right: i18n.t('area_right') };

                        const dbLabel = document.createElement('div');
                        dbLabel.style.cssText = 'font-size: 0.65em; color: #ffda79; margin-top: 4px; font-weight: bold;';
                        dbLabel.textContent = i18n.t('double_baton') || 'DOUBLE BATON';
                        groupDiv.appendChild(dbLabel);

                        const pairGroups = {};
                        doubleBatonPairs.forEach(pair => {
                            const key = pair.areas.sort().join('&');
                            if (!pairGroups[key]) pairGroups[key] = [];
                            pairGroups[key].push(pair);
                        });

                        Object.keys(pairGroups).forEach(key => {
                            const row = document.createElement('div');
                            row.className = 'action-group-buttons grid-3';

                            const areas = key.split('&');
                            areaOrder.forEach(expectedArea => {
                                const pairForPlacement = pairGroups[key].find(p => p.placement === expectedArea);
                                if (pairForPlacement) {
                                    const srcA = areaLabels[areas[0]] || areas[0];
                                    const srcB = areaLabels[areas[1]] || areas[1];
                                    const placeLabel = areaLabels[expectedArea] || expectedArea;

                                    const replaceIndices = areas.map(a => areaIndexMap[a]);
                                    const dbActionParams = {
                                        card_id: firstA.parameters?.card_id,
                                        card_index: firstA.parameters?.card_index,
                                        card_indices: replaceIndices,
                                        stage_area: expectedArea,
                                        use_baton_touch: true,
                                        card_name: firstA.parameters?.card_name,
                                        card_no: firstA.parameters?.card_no,
                                    };

                                    const btn = ActionButtons.createActionButton(
                                        { action_type: 'play_member_to_stage', parameters: dbActionParams },
                                        true, '', State.data
                                    );
                                    btn.innerHTML = `<span style="display:flex;flex-direction:column;align-items:center;gap:1px;font-weight:600;"><span style="font-size:0.7rem;">${srcA}&${srcB}</span><span style="font-size:0.6rem;opacity:0.7;">→${placeLabel} ${pairForPlacement.cost}</span></span>`;
                                    btn.dataset.zoneArea = expectedArea;
                                    btn.style.cssText = '';
                                    btn.className = btn.className + ' action-btn';
                                    btn.onclick = (e) => { e.stopPropagation(); CardDetailModal.close(); if (window.doAction) window.doAction({ action_type: 'play_member_to_stage', parameters: dbActionParams }); };
                                    row.appendChild(btn);
                                } else {
                                    const spacer = document.createElement('div');
                                    spacer.style.cssText = 'flex:1;min-height:36px;border:1px solid transparent;border-right:none;';
                                    row.appendChild(spacer);
                                }
                            });
                            groupDiv.appendChild(row);
                        });
                    }

                    actionsEl.appendChild(groupDiv);
                });

                // Other actions: abilities, system, etc. — use ActionButtons directly
                otherActions.forEach(a => {
                    const btn = ActionButtons.createActionButton(a, false, '', State.data);
                    btn.addEventListener('click', () => CardDetailModal.close(), { capture: true, once: true });
                    actionsEl.appendChild(btn);
                });
            }
        }
    }

    if (footerEl) {
        footerEl.textContent = navCards.length > 1 ? `${navIndex + 1} / ${navCards.length}` : '';
    }
}

export const CardDetailModal = {
    open(card, selectCallback) {
        _selectCallback = selectCallback || null;
        const cards = buildAllCards();
        navCards = [card];
        navIndex = 0;
        navCurrentZone = '';

        // Try to find which zone this card belongs to and set up navigation
        for (const [zone, list] of Object.entries(cards)) {
            const idx = list.findIndex(c => c.card_no === card.card_no || c.id === card.id);
            if (idx >= 0) {
                navCards = list;
                navIndex = idx;
                navCurrentZone = zone;
                break;
            }
        }

        // Show/hide Select button in footer bar
        const selectBtn = document.getElementById('card-detail-select-btn');
        if (selectBtn) {
            if (_selectCallback) {
                selectBtn.style.display = '';
                selectBtn.onclick = () => {
                    const cb = _selectCallback;
                    _selectCallback = null;
                    CardDetailModal.close();
                    if (cb) cb();
                };
            } else {
                selectBtn.style.display = 'none';
            }
        }

        ModalManager.show(DOM_IDS.MODAL_CARD_DETAIL);
        render();
    },

    navigatePrev() {
        if (navCards.length > 1) {
            navIndex = (navIndex - 1 + navCards.length) % navCards.length;
            render();
        }
    },
    navigateNext() {
        if (navCards.length > 1) {
            navIndex = (navIndex + 1) % navCards.length;
            render();
        }
    },
    navigateZonePrev() {
        const cards = buildAllCards();
        const names = Object.keys(cards);
        if (!names.length) return;
        const idx = names.indexOf(navCurrentZone);
        if (idx < 0) { navCurrentZone = names[0]; navCards = cards[names[0]]; navIndex = 0; render(); return; }
        const prev = (idx - 1 + names.length) % names.length;
        navCurrentZone = names[prev];
        navCards = cards[names[prev]];
        navIndex = 0;
        render();
    },
    navigateZoneNext() {
        const cards = buildAllCards();
        const names = Object.keys(cards);
        if (!names.length) return;
        const idx = names.indexOf(navCurrentZone);
        if (idx < 0) { navCurrentZone = names[0]; navCards = cards[names[0]]; navIndex = 0; render(); return; }
        const next = (idx + 1) % names.length;
        navCurrentZone = names[next];
        navCards = cards[names[next]];
        navIndex = 0;
        render();
    },

    close() {
        _selectCallback = null;
        const selectBtn = document.getElementById('card-detail-select-btn');
        if (selectBtn) selectBtn.style.display = 'none';
        ModalManager.hide(DOM_IDS.MODAL_CARD_DETAIL);
    }
};
