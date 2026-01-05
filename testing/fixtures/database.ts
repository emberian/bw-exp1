import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

export interface TestDatabaseConfig {
  path: string;
}

export class TestDatabase {
  private config: TestDatabaseConfig;

  constructor(config: Partial<TestDatabaseConfig> = {}) {
    this.config = {
      path: config.path || path.join(__dirname, '..', 'test.db'),
    };
  }

  /**
   * Remove existing test database to start fresh
   */
  async setup(): Promise<void> {
    // Remove existing test database
    if (fs.existsSync(this.config.path)) {
      fs.unlinkSync(this.config.path);
      console.log(`Removed existing test database: ${this.config.path}`);
    }

    // Also remove any -shm and -wal files
    const shmPath = `${this.config.path}-shm`;
    const walPath = `${this.config.path}-wal`;
    if (fs.existsSync(shmPath)) fs.unlinkSync(shmPath);
    if (fs.existsSync(walPath)) fs.unlinkSync(walPath);

    // The server will auto-run migrations on startup
    console.log('Test database will be created by server on startup');
  }

  /**
   * Run raw SQL against the test database
   */
  async runSql(sql: string): Promise<void> {
    try {
      execSync(`sqlite3 "${this.config.path}" "${sql.replace(/"/g, '\\"')}"`, {
        cwd: path.resolve(__dirname, '../..'),
        stdio: 'pipe',
      });
    } catch (error) {
      console.error('SQL execution failed:', error);
      throw error;
    }
  }

  /**
   * Reset test data between tests (keeps seed data)
   */
  async reset(): Promise<void> {
    // Clean up test-created players and their data
    await this.runSql(`
      DELETE FROM sessions;
      DELETE FROM ships WHERE name LIKE 'test_%';
      DELETE FROM players WHERE username LIKE 'test_%';
    `);
  }

  /**
   * Get database path
   */
  get databasePath(): string {
    return this.config.path;
  }

  /**
   * Get connection string for environment variable
   */
  get connectionString(): string {
    return `sqlite:${this.config.path}`;
  }
}
