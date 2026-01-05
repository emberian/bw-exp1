import { Page } from '@playwright/test';
import { AuthHelper } from './auth';
import { PageWebSocketHelper } from './websocket';

/**
 * Game state helper for setting up specific test scenarios.
 * These helpers interact with the game via API or WebSocket to create
 * specific game states needed for testing.
 */
export class GameStateHelper {
  constructor(
    private page: Page,
    private baseUrl: string,
    private auth: AuthHelper,
    private ws: PageWebSocketHelper
  ) {}

  /**
   * Wait for game to be fully loaded and connected.
   */
  async waitForGameReady(): Promise<void> {
    await this.ws.waitForConnection();
    await this.ws.waitForInitialState();
    // Ensure we have a valid position
    await this.page.waitForSelector('[data-testid="ship-position"]');
  }

  /**
   * Get current ship position from the UI.
   */
  async getShipPosition(): Promise<{ x: number; y: number }> {
    const positionText = await this.page.locator('[data-testid="ship-position"]').textContent();
    if (!positionText) {
      return { x: 0, y: 0 };
    }

    // Parse "X: 123.4 Y: 567.8" format
    const xMatch = positionText.match(/X:\s*([\d.-]+)/);
    const yMatch = positionText.match(/Y:\s*([\d.-]+)/);

    return {
      x: xMatch ? parseFloat(xMatch[1]) : 0,
      y: yMatch ? parseFloat(yMatch[1]) : 0,
    };
  }

  /**
   * Get current ship status from the UI.
   */
  async getShipStatus(): Promise<string> {
    const statusElement = this.page.locator('[data-testid="ship-status"]');
    const text = await statusElement.textContent();
    return text || 'Unknown';
  }

  /**
   * Check if ship is currently docked.
   */
  async isDocked(): Promise<boolean> {
    const status = await this.getShipStatus();
    return status.toLowerCase().includes('docked');
  }

  /**
   * Check if ship is currently in combat.
   */
  async isInCombat(): Promise<boolean> {
    const status = await this.getShipStatus();
    return status.toLowerCase().includes('combat');
  }

  /**
   * Check if ship is moving.
   */
  async isMoving(): Promise<boolean> {
    const status = await this.getShipStatus();
    return status.toLowerCase().includes('transit') || status.toLowerCase().includes('moving');
  }

  /**
   * Wait for ship to reach a position (or timeout).
   */
  async waitForPositionChange(timeout = 10000): Promise<boolean> {
    const initialPos = await this.getShipPosition();

    try {
      await this.page.waitForFunction(
        async (initial) => {
          const posEl = document.querySelector('[data-testid="ship-position"]');
          if (!posEl) return false;
          const text = posEl.textContent || '';
          const xMatch = text.match(/X:\s*([\d.-]+)/);
          const yMatch = text.match(/Y:\s*([\d.-]+)/);
          const x = xMatch ? parseFloat(xMatch[1]) : 0;
          const y = yMatch ? parseFloat(yMatch[1]) : 0;
          return Math.abs(x - initial.x) > 1 || Math.abs(y - initial.y) > 1;
        },
        { x: initialPos.x, y: initialPos.y },
        { timeout }
      );
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Click on sector map at specific position to move ship.
   */
  async moveToMapPosition(x: number, y: number): Promise<void> {
    const sectorMap = this.page.locator('[data-testid="sector-map"]');
    await sectorMap.click({ position: { x, y } });
  }

  /**
   * Stop ship movement via the stop button.
   */
  async stopMovement(): Promise<void> {
    const stopButton = this.page.locator('[data-testid="stop-button"]');
    if (await stopButton.isEnabled()) {
      await stopButton.click();
    }
  }

  /**
   * Try to dock at nearest station.
   */
  async tryDock(): Promise<boolean> {
    const dockButton = this.page.locator('[data-testid="dock-button"]');
    if (await dockButton.isEnabled()) {
      await dockButton.click();
      await this.page.waitForTimeout(1000);
      return await this.isDocked();
    }
    return false;
  }

  /**
   * Undock from current station.
   */
  async undock(): Promise<void> {
    const undockButton = this.page.locator('[data-testid="undock-button"]');
    if (await undockButton.isVisible()) {
      await undockButton.click();
    }
  }

  /**
   * Navigate to a specific tab panel.
   */
  async navigateToTab(tabName: 'Missions' | 'Squadron' | 'Ship' | 'Comms' | 'Combat'): Promise<void> {
    const tab = this.page.locator(`button:has-text("${tabName}")`);
    if (await tab.isVisible()) {
      await tab.click();
    }
  }

  /**
   * Get current fuel level from resource bar.
   */
  async getFuelLevel(): Promise<number> {
    const resourceBar = this.page.locator('[data-testid="resource-bar"]');
    const text = await resourceBar.textContent();
    if (!text) return 0;

    const fuelMatch = text.match(/Fuel\s*([\d.]+)/);
    return fuelMatch ? parseFloat(fuelMatch[1]) : 0;
  }

  /**
   * Get current reputation from resource bar.
   */
  async getReputation(): Promise<number> {
    const resourceBar = this.page.locator('[data-testid="resource-bar"]');
    const text = await resourceBar.textContent();
    if (!text) return 0;

    const repMatch = text.match(/Rep\s*([\d.]+)/);
    return repMatch ? parseFloat(repMatch[1]) : 0;
  }

  /**
   * Get current fame from resource bar.
   */
  async getFame(): Promise<number> {
    const resourceBar = this.page.locator('[data-testid="resource-bar"]');
    const text = await resourceBar.textContent();
    if (!text) return 0;

    const fameMatch = text.match(/Fame\s*([\d.]+)/);
    return fameMatch ? parseFloat(fameMatch[1]) : 0;
  }

  /**
   * Check if there are available missions.
   */
  async hasAvailableMissions(): Promise<boolean> {
    await this.navigateToTab('Missions');
    const missionPanel = this.page.locator('[data-testid="mission-panel"]');
    const acceptButton = missionPanel.locator('button:has-text("Accept")');
    return await acceptButton.isVisible().catch(() => false);
  }

  /**
   * Accept first available mission.
   */
  async acceptFirstMission(): Promise<boolean> {
    await this.navigateToTab('Missions');
    const missionPanel = this.page.locator('[data-testid="mission-panel"]');
    const acceptButton = missionPanel.locator('button:has-text("Accept")').first();

    if (await acceptButton.isVisible().catch(() => false)) {
      await acceptButton.click();
      await this.page.waitForTimeout(500);
      return true;
    }
    return false;
  }

  /**
   * Send a chat message.
   */
  async sendChatMessage(message: string): Promise<void> {
    await this.navigateToTab('Comms');
    const chatInput = this.page.locator('[data-testid="chat-input"]');
    await chatInput.fill(message);
    await chatInput.press('Enter');
  }

  /**
   * Check if station panel is visible (i.e., player is docked).
   */
  async isStationPanelVisible(): Promise<boolean> {
    const stationPanel = this.page.locator('[data-testid="station-panel"]');
    return await stationPanel.isVisible().catch(() => false);
  }

  /**
   * Use a station service.
   */
  async useStationService(service: 'refuel' | 'rearm' | 'repair' | 'shore-leave'): Promise<void> {
    const serviceButton = this.page.locator(`[data-testid="service-${service}"]`);
    if (await serviceButton.isVisible() && await serviceButton.isEnabled()) {
      await serviceButton.click();
    }
  }
}
