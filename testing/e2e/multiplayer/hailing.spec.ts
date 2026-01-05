import { test, expect } from '@playwright/test';
import { MultiplayerHelper } from '../../fixtures/multiplayer';

test.describe('Ship Hailing', () => {
  let mp: MultiplayerHelper;

  test.afterEach(async () => {
    await mp?.cleanup();
  });

  test('should show hail indicator when hailed by another player', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Wait for ships to appear on map
    await alice.page.waitForTimeout(2000);

    // Alice finds Bob on the sector map
    const bobUsername = bob.user.username;
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });

    // Alice selects Bob's ship
    await bobShipMarker.click();

    // Alice clicks Hail button
    const hailButton = alice.page.locator('button:has-text("Hail")');
    await expect(hailButton).toBeVisible();
    await hailButton.click();

    // Bob should see hail indicator - could be on ship marker or as notification
    const hailIndicator = bob.page.locator('.hail-indicator, [data-testid="hail-indicator"]');
    const hailNotification = bob.page.locator('text=/hail/i');

    // At least one of these should be visible
    await expect(hailIndicator.or(hailNotification)).toBeVisible({ timeout: 10000 });
  });

  test('should show hail notification with sender name', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    await alice.page.waitForTimeout(2000);

    const bobUsername = bob.user.username;
    const aliceUsername = alice.user.username;

    // Alice finds and selects Bob's ship
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    // Alice hails Bob
    const hailButton = alice.page.locator('button:has-text("Hail")');
    await expect(hailButton).toBeVisible();
    await hailButton.click();

    // Bob should see notification containing Alice's username
    const notification = bob.page.locator(`text=/hail.*${aliceUsername}|${aliceUsername}.*hail/i`);
    await expect(notification).toBeVisible({ timeout: 10000 });
  });

  test('should allow responding to hail', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    await alice.page.waitForTimeout(2000);

    const bobUsername = bob.user.username;

    // Alice finds and hails Bob
    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const hailButton = alice.page.locator('button:has-text("Hail")');
    await expect(hailButton).toBeVisible();
    await hailButton.click();

    // Bob should see hail notification with respond option
    const hailNotification = bob.page.locator('text=/hail/i');
    await expect(hailNotification).toBeVisible({ timeout: 10000 });

    // Look for respond/acknowledge button
    const respondButton = bob.page.locator('button:has-text(/respond|acknowledge|accept/i)');
    await expect(respondButton).toBeVisible();
    await respondButton.click();

    // Alice should see response indicator
    const responseIndicator = alice.page.locator('text=/responded|acknowledged|accepted/i');
    await expect(responseIndicator).toBeVisible({ timeout: 10000 });
  });
});
