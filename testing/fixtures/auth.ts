import { Page, expect } from '@playwright/test';

export interface TestUser {
  username: string;
  password: string;
  faction: string;
  token?: string;
  playerId?: string;
  shipId?: string;
}

export class AuthHelper {
  constructor(
    private page: Page,
    private baseUrl: string
  ) {}

  /**
   * Register a new test user via API
   */
  async registerUser(user: Partial<TestUser> = {}): Promise<TestUser> {
    const testUser: TestUser = {
      username: user.username || `test_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      password: user.password || 'testpassword123',
      faction: user.faction || 'COMPACT',
    };

    const response = await fetch(`${this.baseUrl}/api/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(testUser),
    });

    const data = await response.json();
    if (!data.success) {
      throw new Error(`Registration failed: ${data.error}`);
    }

    testUser.token = data.token;
    testUser.playerId = data.player_id;
    testUser.shipId = data.ship_id;
    return testUser;
  }

  /**
   * Login user via UI
   */
  async loginViaUI(username: string, password: string): Promise<void> {
    await this.page.goto('/play/login');
    await this.page.fill('input[name="username"], input[type="text"]', username);
    await this.page.fill('input[type="password"]', password);
    await this.page.click('button[type="submit"]');

    // Wait for redirect to game page
    await expect(this.page).toHaveURL(/\/play\/game/, { timeout: 15000 });
  }

  /**
   * Login via API and return token
   */
  async loginViaAPI(username: string, password: string): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    });

    const data = await response.json();
    if (!data.success) {
      throw new Error(`Login failed: ${data.error}`);
    }

    return data.token;
  }

  /**
   * Set auth token in browser localStorage
   */
  async setToken(token: string): Promise<void> {
    await this.page.evaluate((t) => {
      localStorage.setItem('auth_token', t);
    }, token);
  }

  /**
   * Get auth token from browser localStorage
   */
  async getToken(): Promise<string | null> {
    return await this.page.evaluate(() => {
      return localStorage.getItem('auth_token');
    });
  }

  /**
   * Clear auth token from browser localStorage
   */
  async clearToken(): Promise<void> {
    await this.page.evaluate(() => {
      localStorage.removeItem('auth_token');
    });
  }

  /**
   * Create user and set token in browser, ready for game tests
   */
  async createAndLogin(userConfig: Partial<TestUser> = {}): Promise<TestUser> {
    const user = await this.registerUser(userConfig);
    await this.setToken(user.token!);
    return user;
  }

  /**
   * Logout via API
   */
  async logoutViaAPI(token: string): Promise<void> {
    await fetch(`${this.baseUrl}/api/auth/logout`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
    });
  }

  /**
   * Validate token via API
   */
  async validateToken(token: string): Promise<boolean> {
    const response = await fetch(`${this.baseUrl}/api/auth/validate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
    });

    const data = await response.json();
    return data.valid;
  }

  /**
   * Check if user is authenticated (has valid token in localStorage)
   */
  async isAuthenticated(): Promise<boolean> {
    const token = await this.getToken();
    if (!token) return false;
    return this.validateToken(token);
  }
}
