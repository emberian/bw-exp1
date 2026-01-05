import { test, expect } from '@playwright/test';
import { AuthHelper } from '../../fixtures/auth';

test.describe('Registration Flow', () => {
  let auth: AuthHelper;

  test.beforeEach(async ({ page, baseURL }) => {
    auth = new AuthHelper(page, baseURL!);
  });

  test('should display registration page correctly', async ({ page }) => {
    await page.goto('/play/register');

    // Check page title
    await expect(page.locator('h1')).toContainText('Create Your Officer');

    // Check form elements
    await expect(page.locator('[data-testid="username-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="password-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="password-confirm-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="submit-button"]')).toBeVisible();

    // Check navigation link to login
    await expect(page.locator('a[href="/play/login"]')).toBeVisible();
  });

  test('should load and display factions', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load (button elements for faction selection)
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });
  });

  test('should show validation for short username', async ({ page }) => {
    await page.goto('/play/register');

    // Type a short username
    await page.fill('[data-testid="username-input"]', 'ab');

    // Check inline validation hint
    await expect(page.locator('text=more character')).toBeVisible();
  });

  test('should show validation for valid username', async ({ page }) => {
    await page.goto('/play/register');

    // Type a valid username
    await page.fill('[data-testid="username-input"]', 'validuser');

    // Check inline validation hint
    await expect(page.locator('text=Valid callsign')).toBeVisible();
  });

  test('should show validation for short password', async ({ page }) => {
    await page.goto('/play/register');

    // Type a short password
    await page.fill('[data-testid="password-input"]', 'short');

    // Check inline validation hint
    await expect(page.locator('text=more character')).toBeVisible();
  });

  test('should show validation for password mismatch', async ({ page }) => {
    await page.goto('/play/register');

    // Type mismatched passwords
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'differentpassword');

    // Check inline validation hint
    await expect(page.locator("text=Passwords don't match")).toBeVisible();
  });

  test('should show validation for password match', async ({ page }) => {
    await page.goto('/play/register');

    // Type matching passwords
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'validpassword123');

    // Check inline validation hint
    await expect(page.locator('text=Passwords match')).toBeVisible();
  });

  test('should show error for username too short on submit', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Fill with short username
    await page.fill('[data-testid="username-input"]', 'ab');
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'validpassword123');
    await page.click('[data-testid="submit-button"]');

    // Check for error
    await expect(page.locator('[data-testid="error-message"]')).toContainText('at least 3 characters');
  });

  test('should show error for password too short on submit', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Fill with short password
    await page.fill('[data-testid="username-input"]', 'validusername');
    await page.fill('[data-testid="password-input"]', 'short');
    await page.fill('[data-testid="password-confirm-input"]', 'short');
    await page.click('[data-testid="submit-button"]');

    // Check for error
    await expect(page.locator('[data-testid="error-message"]')).toContainText('at least 8 characters');
  });

  test('should show error for password mismatch on submit', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Fill with mismatched passwords
    await page.fill('[data-testid="username-input"]', 'validusername');
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'differentpassword');
    await page.click('[data-testid="submit-button"]');

    // Check for error
    await expect(page.locator('[data-testid="error-message"]')).toContainText('do not match');
  });

  test('should allow faction selection', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Click on a faction to select it
    await page.click('button:has-text("COMPACT")');

    // Should see selection styling (amber border)
    await expect(page.locator('button:has-text("COMPACT")')).toHaveClass(/border-amber/);
  });

  test('should register successfully with valid data', async ({ page }) => {
    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Generate unique username
    const username = `test_reg_${Date.now()}`;

    // Fill valid registration data
    await page.fill('[data-testid="username-input"]', username);
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'validpassword123');

    // Submit
    await page.click('[data-testid="submit-button"]');

    // Should redirect to game page
    await expect(page).toHaveURL(/\/play\/game/, { timeout: 15000 });

    // Check localStorage has token
    const token = await auth.getToken();
    expect(token).toBeTruthy();
  });

  test('should show error for duplicate username', async ({ page, baseURL }) => {
    // First register a user via API
    const existingUser = await auth.registerUser({
      username: `test_dup_${Date.now()}`,
    });

    await page.goto('/play/register');

    // Wait for factions to load
    await expect(page.locator('button:has-text("COMPACT")')).toBeVisible({ timeout: 10000 });

    // Try to register with same username
    await page.fill('[data-testid="username-input"]', existingUser.username);
    await page.fill('[data-testid="password-input"]', 'validpassword123');
    await page.fill('[data-testid="password-confirm-input"]', 'validpassword123');
    await page.click('[data-testid="submit-button"]');

    // Check for error
    await expect(page.locator('[data-testid="error-message"]')).toContainText('already taken');
  });

  test('should navigate to login page from register', async ({ page }) => {
    await page.goto('/play/register');

    // Click login link
    await page.click('a[href="/play/login"]');

    // Should be on login page
    await expect(page).toHaveURL('/play/login');
  });
});
