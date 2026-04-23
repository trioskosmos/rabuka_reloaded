export class DeckValidator {
    constructor() {
        this.registry = null;
        this.isLoading = false;
    }

    async init() {
        if (this.registry) return;
        this.isLoading = true;
        try {
            const resp = await fetch('api/get_card_registry');
            const data = await resp.json();
            if (data.success) {
                this.registry = data.registry;
            }
        } catch (e) {
            console.error("Failed to load card registry", e);
        } finally {
            this.isLoading = false;
        }
    }

    normalizeCode(code) {
        if (!code) return "";
        return code.replace(/＋/g, "+").replace(/－/g, "-").replace(/ー/g, "-").trim().toUpperCase();
    }

    validateDeckString(content) {
        if (!this.registry) return { success: false, error: "Registry not loaded" };

        let main = [];
        let energy = [];
        let isHtml = content.includes('<h3') || content.includes('card-item') || content.includes('title=');

        if (isHtml) {
            // HTML Section Parsing - Try both JP and EN markers for robustness
            let mainHtml = this._extractHtmlSection(content, "メインデッキ") || this._extractHtmlSection(content, "Main Deck");
            let energyHtml = this._extractHtmlSection(content, "エネルギーデッキ") || this._extractHtmlSection(content, "Energy Deck");

            if (mainHtml || energyHtml) {
                main = this._parseCardSection(mainHtml);
                energy = this._parseCardSection(energyHtml);

                // If section parsing found nothing in main, fall back to flat parse
                if (main.length === 0) {
                    main = this._parseCardSection(content);
                }
            } else {
                // Flat HTML fallback
                main = this._parseCardSection(content);
            }
        } else {
            // Plain text parsing
            main = this._parseCardSection(content);
        }

        const results = {
            total: 0,
            members: 0,
            lives: 0,
            energy: 0,
            invalid: [],
            parsed: [], // For Main Deck
            parsedEnergy: []
        };

        const processList = (list, targetArray, isEnergySection) => {
            for (let item of list) {
                const code = item.code;
                const count = item.count;
                const normCode = this.normalizeCode(code);
                const card = this.registry[normCode] || this.registry[code.toUpperCase()];

                if (card) {
                    results.total += count;
                    if (isEnergySection || card.type === 'energy' || normCode.startsWith('LL-E') || normCode.endsWith('-PE') || normCode.endsWith('-PE+')) {
                        results.energy += count;
                    } else {
                        if (card.type === 'member') results.members += count;
                        else if (card.type === 'live') results.lives += count;
                    }

                    targetArray.push({
                        code: normCode,
                        count: count,
                        name: card.name,
                        type: card.type,
                        valid: true,
                        // Support both img, img_path, and _img field names
                        img: card.img || card.img_path || card._img
                    });
                } else {
                    // Skip very short strings or common garbage
                    if (code.length < 4 || /SEARCH|CODE|FAV|COPY/i.test(code)) continue;

                    results.invalid.push(code);
                    targetArray.push({
                        code: code,
                        count: count,
                        valid: false
                    });
                }
            }
        };

        processList(main, results.parsed, false);
        processList(energy, results.parsedEnergy, true);

        return results;
    }

    _extractHtmlSection(html, heading) {
        // More robust heading match: allow some inner spacing/tags if necessary
        const re = new RegExp(`<h3[^>]*>[\\s\\S]*?${heading}[\\s\\S]*?<\\/h3>([\\s\\S]*?)(?=<h3|$)`, 'i');
        const m = html.match(re);
        return m ? m[1] : '';
    }

    _parseCardSection(content) {
        if (!content) return [];
        const result = [];

        // 1. HTML Pattern
        const htmlRe = /title="([^"]+?)\s* : \s*[^"]*"[\s\S]*?<span[^>]*class="num"[^>]*>(\d+)<\/span>/g;
        // Fix for potential HTML variations (note: the above title match is very specific)
        const htmlReAlt = /class="card-item"[\s\S]*?title="([^"]+?)\s*[\s\S]*?<span[^>]*class="num"[^>]*>(\d+)<\/span>/g;
        
        let m;
        let foundAny = false;
        while ((m = htmlRe.exec(content)) !== null) {
            const id = m[1].trim();
            const qty = parseInt(m[2], 10);
            result.push({ code: id, count: qty });
            foundAny = true;
        }

        if (!foundAny) {
            while ((m = htmlReAlt.exec(content)) !== null) {
                const id = m[1].trim();
                const qty = parseInt(m[2], 10);
                result.push({ code: id, count: qty });
                foundAny = true;
            }
        }

        if (foundAny) return result;

        // 2. Text Pattern "ID x N" or "N x ID"
        const lines = content.split(/\r?\n|,\s*/);
        for (let line of lines) {
            line = line.trim();
            if (!line || line.startsWith('#')) continue;

            const suffixMatch = line.match(/^(.+?)\s*[xX]\s*(\d+)$/);
            if (suffixMatch) {
                const id = suffixMatch[1].trim();
                const qty = parseInt(suffixMatch[2]);
                result.push({ code: id, count: qty });
                continue;
            }

            const prefixMatch = line.match(/^(\d+)\s*[xX]\s*(.+)$/);
            if (prefixMatch) {
                const qty = parseInt(prefixMatch[1]);
                const id = prefixMatch[2].trim();
                result.push({ code: id, count: qty });
                continue;
            }

            // Fallback: Just the ID
            result.push({ code: line, count: 1 });
        }

        return result;
    }

    renderPreview(results, targetEl) {
        if (!targetEl) return;

        if (results.parsed.length === 0 && results.parsedEnergy.length === 0) {
            targetEl.innerHTML = `<div class="format-hint">${i18n.t('format_hint')}</div>`;
            return;
        }

        let html = `
            <div class="deck-validation-summary">
                <span class="stat ${results.members === 48 ? 'valid' : 'warning'}">${i18n.t('members')}: ${results.members}/48</span>
                <span class="stat ${results.lives === 12 ? 'valid' : 'warning'}">${i18n.t('lives')}: ${results.lives}/12</span>
                <span class="stat ${results.energy === 12 ? 'valid' : (results.energy > 0 ? 'warning' : '')}">${i18n.t('energy')}: ${results.energy}/12</span>
            </div>
        `;

        if (results.invalid.length > 0) {
            html += `
                <div class="deck-validation-errors">
                    <strong>${i18n.t('invalid_codes')}:</strong>
                    <div class="error-list">${results.invalid.join(', ')}</div>
                </div>
            `;
        }

        html += `<div class="preview-section-title">${i18n.t('main_deck')} (${results.members + results.lives} cards)</div>`;
        html += `<div class="deck-preview-grid">`;
        html += results.parsed.map(p => this._renderItem(p)).join('');
        html += `</div>`;

        if (results.parsedEnergy.length > 0) {
            html += `<div class="preview-section-title">${i18n.t('energy_deck')} (${results.energy} cards)</div>`;
            html += `<div class="deck-preview-grid">`;
            html += results.parsedEnergy.map(p => this._renderItem(p)).join('');
            html += `</div>`;
        }

        targetEl.innerHTML = html;
    }

    _renderItem(p) {
        return `
            <div class="preview-item ${p.img ? 'valid' : 'invalid'}" title="${p.name || 'Unknown'}">
                ${p.img ? `<img src="${p.img}" class="preview-card-img">` : ''}
                <div class="preview-badge">${p.count}x</div>
                <div class="preview-code">${p.code}</div>
            </div>
        `;
    }
}

export const validator = new DeckValidator();
