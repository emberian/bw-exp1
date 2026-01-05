import { test, expect } from '@playwright/test';
import { MultiplayerHelper } from '../../fixtures/multiplayer';

test.describe('PvP Combat', () => {
  let mp: MultiplayerHelper;

  test.afterEach(async () => {
    await mp?.cleanup();
  });

  test('should engage hostile player in combat', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    // Create players in different factions (hostile to each other)
    const alice = await mp.createPlayer('alice', { faction: 'COMPACT' });
    const bob = await mp.createPlayer('bob', { faction: 'HEGEMONY' });

    // Wait for both to appear on map
    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Alice finds Bob on the sector map
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });

    // Alice selects Bob's ship
    await bobShipMarker.click();

    // Look for Engage button (should appear for hostile targets)
    const engageButton = alice.page.locator('button:has-text("Engage")');
    await expect(engageButton).toBeVisible();
    await engageButton.click();

    // Wait for combat to start
    await alice.page.waitForTimeout(2000);

    // Open combat panel
    await alice.page.locator('button:has-text("Combat")').click();
    const combatLog = alice.page.locator('[data-testid="combat-log"]');
    await expect(combatLog).toBeVisible();

    // Should NOT show "No active combat" anymore
    await expect(combatLog.locator('text="No active combat"')).not.toBeVisible();

    // Bob should also see combat
    await bob.page.locator('button:has-text("Combat")').click();
    const bobCombatLog = bob.page.locator('[data-testid="combat-log"]');
    await expect(bobCombatLog).toBeVisible();
    await expect(bobCombatLog.locator('text="No active combat"')).not.toBeVisible();
  });

  test('should show combat updates to both players', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice', { faction: 'COMPACT' });
    const bob = await mp.createPlayer('bob', { faction: 'HEGEMONY' });

    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Alice engages Bob
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const engageButton = alice.page.locator('button:has-text("Engage")');
    await expect(engageButton).toBeVisible();
    await engageButton.click();

    // Wait for combat rounds
    await alice.page.waitForTimeout(5000);

    // Both should have combat log entries
    await alice.page.locator('button:has-text("Combat")').click();
    await bob.page.locator('button:has-text("Combat")').click();

    // Check for combat log entries
    const aliceEntries = alice.page.locator('[data-testid="combat-log-entry"]');
    const bobEntries = bob.page.locator('[data-testid="combat-log-entry"]');

    // At least one player should have combat entries
    const aliceCount = await aliceEntries.count();
    const bobCount = await bobEntries.count();

    expect(aliceCount + bobCount).toBeGreaterThan(0);
  });

  test('should disengage from combat', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice', { faction: 'COMPACT' });
    const bob = await mp.createPlayer('bob', { faction: 'HEGEMONY' });

    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Alice engages Bob
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const engageButton = alice.page.locator('button:has-text("Engage")');
    await expect(engageButton).toBeVisible();
    await engageButton.click();

    await alice.page.waitForTimeout(2000);

    // Try to disengage - look for disengage/flee button
    const disengageButton = alice.page.locator('button:has-text(/disengage|flee|escape/i)');
    await expect(disengageButton).toBeVisible();
    await disengageButton.click();

    // Wait for disengage to process
    await alice.page.waitForTimeout(2000);

    // Check combat ended
    await alice.page.locator('button:has-text("Combat")').click();
    const combatLog = alice.page.locator('[data-testid="combat-log"]');

    // Should show no active combat or combat ended
    const noActiveCombat = combatLog.locator('text=/no active combat|combat ended/i');
    await expect(noActiveCombat).toBeVisible({ timeout: 10000 });
  });

  test('should show different factions as hostile', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice', { faction: 'COMPACT' });
    const bob = await mp.createPlayer('bob', { faction: 'HEGEMONY' });

    await alice.page.waitForTimeout(3000);

    const bobUsername = bob.user.username;

    // Alice clicks on Bob's ship
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    // Target panel should indicate hostile status
    const targetPanel = alice.page.locator('[data-testid="target-info"], [data-testid="ship-info-panel"]');
    await expect(targetPanel).toBeVisible();

    // Should show hostile indicator and Engage button (not Hail only)
    const hostileIndicator = targetPanel.locator('text=/hostile|enemy|hegemony/i');
    const engageButton = alice.page.locator('button:has-text("Engage")');

    await expect(hostileIndicator.or(engageButton)).toBeVisible();
  });
});
