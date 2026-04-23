/**
 * Log Performance Monitoring
 */
export const PerformanceMonitor = {
    _perfMetrics: {
        renderTime: 0,
        entryCount: 0,
        filterTime: 0,
        lastRenderTimestamp: 0
    },

    _lastLogCount: 0,
    _lastHistoryCount: 0,

    startPerfMeasure: () => {
        PerformanceMonitor._perfMetrics.renderTime = performance.now();
        PerformanceMonitor._perfMetrics.entryCount = 0;
        return PerformanceMonitor._perfMetrics.renderTime;
    },

    endPerfMeasure: () => {
        const duration = performance.now() - PerformanceMonitor._perfMetrics.renderTime;
        PerformanceMonitor._perfMetrics.renderTime = duration;
        PerformanceMonitor.checkPerformance();
        return duration;
    },

    recordEntryCount: (count) => {
        PerformanceMonitor._perfMetrics.entryCount += count;
    },

    getPerfReport: () => {
        return {
            renderTime: PerformanceMonitor._perfMetrics.renderTime,
            entryCount: PerformanceMonitor._perfMetrics.entryCount,
            filterTime: PerformanceMonitor._perfMetrics.filterTime,
            avgRenderTime: PerformanceMonitor._perfMetrics.entryCount > 0
                ? PerformanceMonitor._perfMetrics.renderTime / PerformanceMonitor._perfMetrics.entryCount
                : 0
        };
    },

    checkPerformance: (threshold = 100) => {
        const report = PerformanceMonitor.getPerfReport();
        if (report.renderTime > threshold) {
            console.warn(`[Log Performance] Render took ${report.renderTime.toFixed(2)}ms (threshold: ${threshold}ms)`);
            return true;
        }
        return false;
    }
};
