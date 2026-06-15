import { Network } from '../network.js';

export const ReportModal = {
    openReportModal: () => {
        const modal = document.getElementById('report-modal');
        if (modal) modal.style.display = 'flex';
    },

    closeReportModal: () => {
        const modal = document.getElementById('report-modal');
        if (modal) modal.style.display = 'none';
    },

    submitReport: async () => ReportModal.downloadReport(),

    downloadReport: async () => {
        const explanation = document.getElementById('report-explanation').value;
        if (!explanation) {
            alert("Please provide an explanation of the issue.");
            return;
        }

        const reportData = await Network.buildDownloadReport(explanation);

        const blob = new Blob([JSON.stringify(reportData, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        const ts = new Date().toISOString().replace(/[:.]/g, '-');
        a.download = `lovecasim_report_${ts}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        document.getElementById('report-explanation').value = "";
        ReportModal.closeReportModal();
    }
};
