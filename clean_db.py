import sqlite3
import os
import re

db_path = os.path.expanduser("~/Library/Application Support/com.bongpark.lumina-mail/lumina_mail.db")
conn = sqlite3.connect(db_path)
cur = conn.cursor()

cur.execute("SELECT id, body_summary FROM messages")
rows = cur.fetchall()

count = 0
for row in rows:
    msg_id, summary = row[0], row[1]
    if summary and '{' in summary and '}' in summary:
        # Regex to strip common CSS rules: selector { styles }
        clean = re.sub(r'[\w\s,>+~:#.-]+\s*\{[^}]*\}', ' ', summary)
        # Second pass for nested or multiple
        clean = re.sub(r'[\w\s,>+~:#.-]+\s*\{[^}]*\}', ' ', clean)
        clean = re.sub(r'\s+', ' ', clean).strip()
        
        if clean != summary:
            cur.execute("UPDATE messages SET body_summary = ? WHERE id = ?", (clean, msg_id))
            count += 1

conn.commit()
conn.close()

print(f"Cleaned {count} legacy CSS-infected messages.")
