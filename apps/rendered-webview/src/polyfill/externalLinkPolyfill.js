class TauriExternalLinksPolyfill {
    openerPlugin = null;
    originalWindowOpen = null;
    isInitialized = false;
  
    constructor() {
      this.init();
    }
  
    /**
     * Initialize the polyfill
     */
    async init() {
      if (this.isInitialized) return;
  
      await this.loadOpenerPlugin();
      this.originalWindowOpen = window.open;
      this.overrideWindowOpen();
      this.isInitialized = true;
    }
  
    /**
     * Load the Tauri opener plugin
     */
    async loadOpenerPlugin() {
      try {
        const opener = await import('@tauri-apps/plugin-opener');
        this.openerPlugin = opener;
      } catch (error) {
        console.error('Failed to load opener plugin:', error);
      }
    }
  
    /**
     * Determine if URL is external
     * @param {string} url - The URL to check
     * @returns {boolean} - True if external, false otherwise
     */
    isExternalUrl(url) {
      if (!url) return false;
      
      try {
        const urlObj = new URL(url, window.location.href);
        const currentHost = window.location.host;
        
        return (
          urlObj.protocol === 'http:' || 
          urlObj.protocol === 'https:'
        ) && urlObj.host !== currentHost;
      } catch (e) {
        return /^https?:\/\//.test(url);
      }
    }
  
    /**
     * Open external URL using Tauri opener plugin
     * @param {string} url - The URL to open
     * @returns {Promise<boolean>} - True if successful, false otherwise
     */
    async openExternalUrl(url) {
      try {
        if (this.openerPlugin && this.openerPlugin.openUrl) {
          await this.openerPlugin.openUrl(url);
          return true;
        } else {
          console.error('Opener plugin not available');
        }
      } catch (error) {
        console.error('Failed to open URL with Tauri opener plugin:', error);
      }
      
      return false;
    }
  
    /**
     * Override window.open to intercept external URLs
     */
    overrideWindowOpen() {
      const self = this;
      
      window.open = function(url, target, features) {
        // Handle external URLs
        if (self.isExternalUrl(url)) {
          console.log('Intercepting window.open for external URL:', url);
          
          self.openExternalUrl(url).then(success => {
            if (!success) {
              console.warn('Opener plugin failed, falling back to original window.open');
              self.originalWindowOpen.call(window, url, target, features);
            }
          }).catch(error => {
            console.error('Error in window.open polyfill:', error);
            self.originalWindowOpen.call(window, url, target, features);
          });
          
          // Return a mock window object to prevent errors
          return self.createMockWindow();
        }
        
        // For internal URLs, use original window.open
        return self.originalWindowOpen.call(window, url, target, features);
      };
    }
  
    /**
     * Create a mock window object for window.open return value
     * @returns {Object} - Mock window object
     */
    createMockWindow() {
      return {
        closed: false,
        close: () => {},
        focus: () => {},
        blur: () => {},
        postMessage: () => {}
      };
    }
  }
  
  // Auto-initialize when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => new TauriExternalLinksPolyfill());
  } else {
    new TauriExternalLinksPolyfill();
  }