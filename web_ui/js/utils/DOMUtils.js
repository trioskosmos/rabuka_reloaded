/**
 * DOM Utilities
 * Standardized helpers for common DOM manipulation patterns
 */

export const DOMUtils = {
  /**
   * Get element by ID with safety check
   * @param {string} id - Element ID
   * @returns {HTMLElement|null}
   */
  getElement: (id) => {
    const el = document.getElementById(id);
    if (!el) {
      console.warn(`[DOMUtils] Element not found: ${id}`);
    }
    return el;
  },

  /**
   * Get multiple elements by IDs
   * @param {string[]} ids - Array of element IDs
   * @returns {Map<string, HTMLElement>}
   */
  getElements: (ids) => {
    const map = new Map();
    ids.forEach(id => {
      map.set(id, DOMUtils.getElement(id));
    });
    return map;
  },

  /**
   * Clear an element's content
   * @param {string} id - Element ID
   */
  clear: (id) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.innerHTML = '';
    }
  },

  /**
   * Set text content safely (prevents XSS)
   * @param {string} id - Element ID
   * @param {string|number} text - Text to set
   */
  setText: (id, text) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.textContent = String(text);
    }
  },

  /**
   * Set HTML content (use only for trusted content)
   * @param {string} id - Element ID
   * @param {string} html - HTML to set
   */
  setHTML: (id, html) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.innerHTML = html;
    }
  },

  /**
   * Render content to element (clears first)
   * @param {string} id - Element ID
   * @param {string|HTMLElement|DocumentFragment} content - Content to render
   */
  render: (id, content) => {
    const el = DOMUtils.getElement(id);
    if (!el) return;
    
    el.innerHTML = '';
    
    if (typeof content === 'string') {
      el.innerHTML = content;
    } else if (content instanceof DocumentFragment || content instanceof HTMLElement) {
      el.appendChild(content);
    }
  },

  /**
   * Batch update multiple element text values
   * @param {Object} updates - { elementId: textValue, ... }
   */
  updateText: (updates) => {
    Object.entries(updates).forEach(([id, text]) => {
      DOMUtils.setText(id, text);
    });
  },

  /**
   * Add class to element
   * @param {string} id - Element ID
   * @param {string} className - Class name to add
   */
  addClass: (id, className) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.classList.add(className);
    }
  },

  /**
   * Remove class from element
   * @param {string} id - Element ID
   * @param {string} className - Class name to remove
   */
  removeClass: (id, className) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.classList.remove(className);
    }
  },

  /**
   * Toggle class on element
   * @param {string} id - Element ID
   * @param {string} className - Class name to toggle
   */
  toggleClass: (id, className) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.classList.toggle(className);
    }
  },

  /**
   * Set element style property
   * @param {string} id - Element ID
   * @param {string} property - CSS property name
   * @param {string} value - CSS property value
   */
  setStyle: (id, property, value) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      el.style[property] = value;
    }
  },

  /**
   * Set multiple style properties
   * @param {string} id - Element ID
   * @param {Object} styles - { property: value, ... }
   */
  setStyles: (id, styles) => {
    const el = DOMUtils.getElement(id);
    if (el) {
      Object.entries(styles).forEach(([prop, val]) => {
        el.style[prop] = val;
      });
    }
  },

  /**
   * Get element style value
   * @param {string} id - Element ID
   * @param {string} property - CSS property name
   */
  getStyle: (id, property) => {
    const el = DOMUtils.getElement(id);
    if (!el) return null;
    return el.style[property];
  },

  /**
   * Set element display value directly.
   * @param {string} id - Element ID
   * @param {string} value - CSS display value
   */
  setDisplay: (id, value) => {
    DOMUtils.setStyle(id, 'display', value);
  },

  /**
   * Toggle element visibility with an explicit display mode when shown.
   * @param {string} id - Element ID
   * @param {boolean} visible - Whether the element should be visible
   * @param {string} shownDisplayValue - Display value used when visible
   */
  setVisible: (id, visible, shownDisplayValue = '') => {
    DOMUtils.setDisplay(id, visible ? shownDisplayValue : 'none');
  },

  /**
   * Patch attributes of an element (only updates if changed)
   * @param {HTMLElement} el - Element to patch
   * @param {Object} attrs - { attribute: value, ... }
   */
  patchAttributes: (el, attrs) => {
    if (!el) return;
    Object.entries(attrs).forEach(([name, value]) => {
      if (value === undefined || value === null) {
        if (el.hasAttribute(name)) el.removeAttribute(name);
      } else if (el.getAttribute(name) !== String(value)) {
        el.setAttribute(name, String(value));
      }
    });
  },

  /**
   * Patch classes of an element (only updates if changed)
   * @param {HTMLElement} el - Element to patch
   * @param {string} classString - New class string
   */
  patchClasses: (el, classString) => {
    if (!el || el.className === classString) return;
    el.className = classString;
  },

  /**
   * Check if element has class
   * @param {string} id - Element ID
   * @param {string} className - Class name
   */
  hasClass: (id, className) => {
    const el = DOMUtils.getElement(id);
    if (!el) return false;
    return el.classList.contains(className);
  },

  /**
   * Set element cursor style
   * @param {string} id - Element ID
   * @param {string} cursor - Cursor style
   */
  setCursor: (id, cursor) => {
    DOMUtils.setStyle(id, 'cursor', cursor);
  },

  /**
   * Set element background color
   * @param {string} id - Element ID
   * @param {string} color - Color value
   */
  setBackground: (id, color) => {
    DOMUtils.setStyle(id, 'background', color);
  },

  /**
   * Set element text color
   * @param {string} id - Element ID
   * @param {string} color - Color value
   */
  setColor: (id, color) => {
    DOMUtils.setStyle(id, 'color', color);
  },

  /**
   * Show element
   * @param {string} id - Element ID
   */
  show: (id) => {
    DOMUtils.setDisplay(id, '');
  },

  /**
   * Hide element
   * @param {string} id - Element ID
   */
  hide: (id) => {
    DOMUtils.setDisplay(id, 'none');
  },

  /**
   * Check if element exists
   * @param {string} id - Element ID
   */
  exists: (id) => {
    return document.getElementById(id) !== null;
  },
};
