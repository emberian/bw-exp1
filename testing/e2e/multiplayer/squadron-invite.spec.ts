import { test, expect } from '@playwright/test';
import { MultiplayerHelper } from '../../fixtures/multiplayer';

test.describe('Squadron Invitation Flow', () => {
  let mp: MultiplayerHelper;

  test.afterEach(async () => {
    await mp?.cleanup();
  });

  test('should send and accept squadron invite', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Alice creates a squadron
    await alice.page.locator('button:has-text("Squadron")').click();
    await expect(alice.page.locator('[data-testid="squadron-panel"]')).toBeVisible();

    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('TestSquad');
    await alice.page.locator('input[placeholder*="tag" i]').fill('TST');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    await expect(alice.page.locator('[data-testid="squadron-panel"]')).toContainText('TestSquad');

    // Alice finds Bob on the sector map and invites him
    const bobUsername = bob.user.username;
    await alice.page.waitForTimeout(2000); // Wait for Bob to appear on map

    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    // Look for invite button in target panel
    const inviteButton = alice.page.locator('button:has-text(/invite.*squadron|squadron.*invite/i)');
    await expect(inviteButton).toBeVisible();
    await inviteButton.click();

    // Bob should receive the invitation
    const bobNotification = bob.page.locator('[data-testid="notifications-panel"]');
    await expect(bobNotification).toBeVisible({ timeout: 10000 });

    // Bob accepts
    await bob.page.locator('button:has-text("Accept")').first().click();

    // Verify Bob is now in the squadron
    await bob.page.locator('button:has-text("Squadron")').click();
    await expect(bob.page.locator('[data-testid="squadron-panel"]')).toContainText('TestSquad');
  });

  test('should send and decline squadron invite', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Alice creates squadron
    await alice.page.locator('button:has-text("Squadron")').click();
    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('DeclineTest');
    await alice.page.locator('input[placeholder*="tag" i]').fill('DCL');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    // Alice invites Bob
    const bobUsername = bob.user.username;
    await alice.page.waitForTimeout(2000);

    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const inviteButton = alice.page.locator('button:has-text(/invite.*squadron|squadron.*invite/i)');
    await expect(inviteButton).toBeVisible();
    await inviteButton.click();

    // Bob receives and declines
    const bobNotification = bob.page.locator('[data-testid="notifications-panel"]');
    await expect(bobNotification).toBeVisible({ timeout: 10000 });

    await bob.page.locator('button:has-text("Decline")').first().click();

    // Verify Bob is NOT in a squadron
    await bob.page.locator('button:has-text("Squadron")').click();
    const squadronPanel = bob.page.locator('[data-testid="squadron-panel"]');
    await expect(squadronPanel).not.toContainText('DeclineTest');
  });

  test('should show invite notification with squadron name', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Alice creates squadron
    await alice.page.locator('button:has-text("Squadron")').click();
    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('NotifySquad');
    await alice.page.locator('input[placeholder*="tag" i]').fill('NTF');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    // Alice invites Bob
    const bobUsername = bob.user.username;
    await alice.page.waitForTimeout(2000);

    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const inviteButton = alice.page.locator('button:has-text(/invite.*squadron|squadron.*invite/i)');
    await expect(inviteButton).toBeVisible();
    await inviteButton.click();

    // Bob's notification should contain squadron name
    const bobNotification = bob.page.locator('[data-testid="notifications-panel"]');
    await expect(bobNotification).toBeVisible({ timeout: 10000 });
    await expect(bobNotification).toContainText('NotifySquad');
  });

  test('should allow member to leave squadron', async ({ browser, baseURL }) => {
    mp = new MultiplayerHelper(browser, baseURL!);

    const alice = await mp.createPlayer('alice');
    const bob = await mp.createPlayer('bob');

    // Alice creates squadron
    await alice.page.locator('button:has-text("Squadron")').click();
    const createButton = alice.page.locator('[data-testid="squadron-show-create-button"]');
    await expect(createButton).toBeVisible();
    await createButton.click();

    await alice.page.locator('input[placeholder*="name" i]').fill('LeaveTest');
    await alice.page.locator('input[placeholder*="tag" i]').fill('LVT');
    await alice.page.locator('[data-testid="squadron-create-button"]').click();

    // Alice invites Bob
    const bobUsername = bob.user.username;
    await alice.page.waitForTimeout(2000);

    const bobShipMarker = alice.page.locator(`text=${bobUsername}`).first();
    await expect(bobShipMarker).toBeVisible({ timeout: 10000 });
    await bobShipMarker.click();

    const inviteButton = alice.page.locator('button:has-text(/invite.*squadron|squadron.*invite/i)');
    await expect(inviteButton).toBeVisible();
    await inviteButton.click();

    // Bob accepts
    const bobNotification = bob.page.locator('[data-testid="notifications-panel"]');
    await expect(bobNotification).toBeVisible({ timeout: 10000 });
    await bob.page.locator('button:has-text("Accept")').first().click();

    // Verify Bob joined
    await bob.page.locator('button:has-text("Squadron")').click();
    await expect(bob.page.locator('[data-testid="squadron-panel"]')).toContainText('LeaveTest');

    // Bob leaves
    const leaveButton = bob.page.locator('button:has-text("Leave")');
    await expect(leaveButton).toBeVisible();
    await leaveButton.click();

    // Verify Bob is no longer in squadron
    await expect(bob.page.locator('[data-testid="squadron-panel"]')).not.toContainText('LeaveTest');
  });
});
