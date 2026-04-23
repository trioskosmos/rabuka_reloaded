/**
 * Log Filter Utilities
 */
export const LogFilter = {
    filterState: {
        eventTypes: new Set(['PLAY', 'ACTIVATE', 'TRIGGER', 'EFFECT', 'RULE', 'YELL', 'PERFORMANCE']),
        players: new Set([0, 1]),
        searchText: '',
        selectedTurn: -1
    },

    applyFilters: (events) => {
        const filters = LogFilter.filterState;
        return events.filter(e => {
            if (!filters.eventTypes.has(e.event_type)) return false;
            if (!filters.players.has(e.player_id)) return false;
            if (filters.selectedTurn !== -1 && e.turn !== filters.selectedTurn) return false;
            if (filters.searchText && !e.description.toLowerCase().includes(filters.searchText.toLowerCase())) return false;
            return true;
        });
    },

    toggleEventType: (eventType) => {
        if (LogFilter.filterState.eventTypes.has(eventType)) {
            LogFilter.filterState.eventTypes.delete(eventType);
        } else {
            LogFilter.filterState.eventTypes.add(eventType);
        }
    },

    togglePlayer: (playerId) => {
        if (LogFilter.filterState.players.has(playerId)) {
            LogFilter.filterState.players.delete(playerId);
        } else {
            LogFilter.filterState.players.add(playerId);
        }
    },

    setSearchText: (text) => {
        LogFilter.filterState.searchText = text;
    },

    setTurnFilter: (turn) => {
        LogFilter.filterState.selectedTurn = turn;
    },

    resetFilters: () => {
        LogFilter.filterState = {
            eventTypes: new Set(['PLAY', 'ACTIVATE', 'TRIGGER', 'EFFECT', 'RULE', 'YELL', 'PERFORMANCE']),
            players: new Set([0, 1]),
            searchText: '',
            selectedTurn: -1
        };
    },

    renderFilterControls: (container, t, onChangeCallback) => {
        const filterDiv = document.createElement('div');
        filterDiv.className = 'log-filter-controls';

        const eventTypes = ['PLAY', 'ACTIVATE', 'TRIGGER', 'EFFECT', 'RULE', 'YELL'];
        const eventTypeHtml = eventTypes.map(type => {
            const checked = LogFilter.filterState.eventTypes.has(type) ? 'checked' : '';
            const label = t[`event_${type.toLowerCase()}`] || type;
            return `<label class="filter-checkbox">
                <input type="checkbox" value="${type}" ${checked} onchange="Logs.toggleEventType('${type}')">
                ${label}
            </label>`;
        }).join('');

        filterDiv.innerHTML = `
            <div class="filter-row">
                <input type="text" class="form-input log-search-input"
                    placeholder="${t['search_placeholder'] || 'Search...'}"
                    value="${LogFilter.filterState.searchText}"
                    oninput="Logs.setSearchText(this.value)">
            </div>
            <div class="filter-row event-type-filters">
                ${eventTypeHtml}
            </div>
        `;

        container.appendChild(filterDiv);
    }
};
