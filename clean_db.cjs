const sqlite3 = require('sqlite3').verbose();
const path = require('path');
const os = require('os');
const dbPath = path.join(os.homedir(), 'Library/Application Support/com.bongpark.lumina-mail/lumina_mail.db');

const db = new sqlite3.Database(dbPath);

console.log("Connected to local DB:", dbPath);

db.serialize(() => {
  db.all("SELECT id, body_summary FROM messages", (err, rows) => {
    if (err) throw err;
    let count = 0;
    
    // Begin transaction
    db.run("BEGIN TRANSACTION");
    const stmt = db.prepare("UPDATE messages SET body_summary = ? WHERE id = ?");
    
    for (const row of rows) {
      if (row.body_summary && row.body_summary.includes('{') && row.body_summary.includes('}')) {
        let clean = row.body_summary.replace(/[\w\s,>+~:#.-]+\s*\{[^}]*\}/g, ' ');
        // Second pass for nested or multiple blocks
        clean = clean.replace(/[\w\s,>+~:#.-]+\s*\{[^}]*\}/g, ' ');
        clean = clean.replace(/\s+/g, ' ').trim();
        
        if (clean !== row.body_summary) {
          stmt.run(clean, row.id);
          count++;
        }
      }
    }
    
    stmt.finalize(() => {
      db.run("COMMIT", () => {
        console.log(`Cleaned ${count} legacy CSS-infected messages.`);
        db.close();
      });
    });
  });
});
