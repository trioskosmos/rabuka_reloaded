/**
 * String Utility Module
 * Centralized string manipulation and cleaning logic
 */

export const StringUtils = {
  /**
   * Cleans card names by removing bracketed technical metadata
   * @param {string} name - The raw card name
   * @returns {string} The cleaned card name
   */
  cleanCardName: (name) => {
    if (!name) return "";
    // Only removes specific common technical metadata markers (rarity/type)
    // to avoid stripping intentional descriptive brackets from action labels.
    const technicalTags = /(?:[【\[](?:UR|SR|R|N|PR|P|BR|SEC|Promo|PROMO|限定|特製|サイン|非売品)[】\]])/g;
    return name.replace(technicalTags, "").trim();
  }
};
