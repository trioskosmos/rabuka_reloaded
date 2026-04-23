/**
 * Centralized Modal Manager
 * Handles all modal visibility, display logic, and event delegation
 */
import { DOM_IDS, DISPLAY_VALUES } from '../constants_dom.js';

export const ModalManager = {
  /**
   * Show a modal element
   * @param {string} modalId - DOM ID of the modal
   * @param {string} displayValue - CSS display value (default: 'flex')
   */
  show: (modalId, displayValue = DISPLAY_VALUES.FLEX) => {
    const modal = document.getElementById(modalId);
    if (!modal) {
      console.warn(`[ModalManager] Modal not found: ${modalId}`);
      return false;
    }
    modal.style.display = displayValue;
    return true;
  },

  /**
   * Hide a modal element
   * @param {string} modalId - DOM ID of the modal
   */
  hide: (modalId) => {
    const modal = document.getElementById(modalId);
    if (!modal) {
      console.warn(`[ModalManager] Modal not found: ${modalId}`);
      return false;
    }
    modal.style.display = DISPLAY_VALUES.NONE;
    return true;
  },

  /**
   * Toggle modal visibility
   * @param {string} modalId - DOM ID of the modal
   * @param {string} showValue - Display value when shown
   */
  toggle: (modalId, showValue = DISPLAY_VALUES.FLEX) => {
    const modal = document.getElementById(modalId);
    if (!modal) {
      console.warn(`[ModalManager] Modal not found: ${modalId}`);
      return null;
    }
    const isHidden = modal.style.display === DISPLAY_VALUES.NONE;
    if (isHidden) {
      modal.style.display = showValue;
    } else {
      modal.style.display = DISPLAY_VALUES.NONE;
    }
    return !isHidden;
  },

  /**
   * Hide a modal-like element directly.
   * @param {HTMLElement} modal - Element to hide
   */
  hideElement: (modal) => {
    if (!modal) {
      return false;
    }
    modal.style.display = DISPLAY_VALUES.NONE;
    return true;
  },

  /**
   * Set up auto-close on outside click (backdrop click)
   * @param {string} modalId - Modal ID
   * @param {Function} onClose - Optional callback when closed
   */
  setupBackdropClose: (modalId, onClose = null) => {
    const modal = document.getElementById(modalId);
    if (!modal) {
      console.warn(`[ModalManager] Modal not found: ${modalId}`);
      return;
    }
    
    modal.addEventListener('click', (e) => {
      if (e.target === modal) {
        ModalManager.hide(modalId);
        if (onClose && typeof onClose === 'function') {
          onClose();
        }
      }
    });
  },

  /**
   * Get current display state
   * @param {string} modalId - Modal ID
   */
  isVisible: (modalId) => {
    const modal = document.getElementById(modalId);
    if (!modal) return false;
    return modal.style.display !== DISPLAY_VALUES.NONE && window.getComputedStyle(modal).display !== DISPLAY_VALUES.NONE;
  },
};
