import { Database } from 'bun:sqlite';

const db = new Database('./data/analytics.db');

console.log('Adding IP address column to sessions table...');

try {
  db.exec(`ALTER TABLE sessions ADD COLUMN ip_address TEXT`);
  console.log('Added ip_address column to sessions table');
} catch (e) {
  if (e.message.includes('duplicate column')) {
    console.log('ip_address column already exists');
  } else {
    console.error('Error adding column:', e.message);
  }
}

db.close();
console.log('Migration complete');