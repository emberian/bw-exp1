import { Browser, BrowserContext, Page } from '@playwright/test';
import { AuthHelper, TestUser } from './auth';
import { PageWebSocketHelper } from './websocket';

/**
 * Represents a single player's browser context and helpers.
 */
export interface PlayerContext {
  /** Isolated browser context for this player */
  context: BrowserContext;
  /** Page within the context */
  page: Page;
  /** User credentials and IDs */
  user: TestUser;
  /** Auth helper for this player's page */
  auth: AuthHelper;
  /** WebSocket helper for this player's page */
  ws: PageWebSocketHelper;
}

/**
 * Options for creating a player.
 */
export interface CreatePlayerOptions {
  /** Faction for the player (affects hostility) */
  faction?: 'COMPACT' | 'HEGEMONY' | 'INDEPENDENTS';
  /** Custom username prefix */
  usernamePrefix?: string;
}

/**
 * Helper for managing multiple players in E2E tests.
 *
 * Creates isolated browser contexts for each player, allowing
 * testing of player-to-player interactions like chat, squadrons,
 * and combat.
 *
 * @example
 * ```typescript
 * test('squadron invite flow', async ({ browser, baseURL }) => {
 *   const mp = new MultiplayerHelper(browser, baseURL!);
 *
 *   const alice = await mp.createPlayer('alice');
 *   const bob = await mp.createPlayer('bob');
 *
 *   // Alice creates squadron, invites Bob...
 *
 *   await mp.cleanup();
 * });
 * ```
 */
export class MultiplayerHelper {
  private players: Map<string, PlayerContext> = new Map();

  constructor(
    private browser: Browser,
    private baseUrl: string
  ) {}

  /**
   * Create a new player with their own browser context.
   *
   * @param name - Identifier for this player (e.g., 'alice', 'bob')
   * @param options - Optional configuration for the player
   * @returns PlayerContext with page, user info, and helpers
   */
  async createPlayer(name: string, options: CreatePlayerOptions = {}): Promise<PlayerContext> {
    // Create isolated browser context
    const context = await this.browser.newContext();
    const page = await context.newPage();

    // Set up helpers
    const auth = new AuthHelper(page, this.baseUrl);
    const ws = new PageWebSocketHelper(page);

    // Generate unique username (max 24 chars)
    // Use last 4 digits of timestamp + 3 char random = short but unique
    const prefix = (options.usernamePrefix || name).slice(0, 10);
    const suffix = `${Date.now() % 10000}_${Math.random().toString(36).slice(2, 5)}`;
    const username = `${prefix}_${suffix}`;

    // Register and login
    const user = await auth.createAndLogin({
      username,
      faction: options.faction || 'COMPACT',
    });

    // Navigate to game and wait for connection
    await page.goto('/play/game');
    await ws.waitForConnection();
    await ws.waitForInitialState();

    const player: PlayerContext = { context, page, user, auth, ws };
    this.players.set(name, player);
    return player;
  }

  /**
   * Get a player by name.
   *
   * @param name - Player identifier
   * @returns PlayerContext or undefined if not found
   */
  get(name: string): PlayerContext | undefined {
    return this.players.get(name);
  }

  /**
   * Get a player by name, throwing if not found.
   *
   * @param name - Player identifier
   * @returns PlayerContext
   * @throws Error if player not found
   */
  getRequired(name: string): PlayerContext {
    const player = this.players.get(name);
    if (!player) {
      throw new Error(`Player '${name}' not found. Did you call createPlayer('${name}')?`);
    }
    return player;
  }

  /**
   * Get all players.
   *
   * @returns Array of [name, PlayerContext] pairs
   */
  getAll(): [string, PlayerContext][] {
    return Array.from(this.players.entries());
  }

  /**
   * Clean up all player contexts.
   * Call this in afterEach or at the end of tests.
   */
  async cleanup(): Promise<void> {
    for (const [, player] of this.players) {
      await player.context.close();
    }
    this.players.clear();
  }

  /**
   * Wait for a condition on one player's page that depends on another player's action.
   * Useful for testing cross-player events with proper timeouts.
   *
   * @param playerName - Player whose page to check
   * @param condition - Async function that returns true when condition is met
   * @param timeout - Maximum time to wait in ms (default 10000)
   */
  async waitForCrossPlayerEvent(
    playerName: string,
    condition: (page: Page) => Promise<boolean>,
    timeout = 10000
  ): Promise<void> {
    const player = this.getRequired(playerName);
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      if (await condition(player.page)) {
        return;
      }
      await player.page.waitForTimeout(100);
    }

    throw new Error(`Timeout waiting for cross-player event on '${playerName}'`);
  }
}
