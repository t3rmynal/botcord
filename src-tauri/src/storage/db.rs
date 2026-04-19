use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn with<R>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<R>) -> rusqlite::Result<R> {
        let c = self.conn.lock().unwrap();
        f(&c)
    }

    pub fn migrate(&self) -> rusqlite::Result<()> {
        self.with(|c| {
            c.execute_batch(
                r#"
                create table if not exists settings (
                  key text primary key,
                  value text not null
                );

                create table if not exists accounts (
                  id text primary key,
                  discord_id text,
                  label text,
                  token_enc blob not null,
                  token_nonce blob not null,
                  proxy_id text,
                  valid integer,
                  last_check_at integer,
                  meta_json text,
                  created_at integer not null default (strftime('%s','now'))
                );

                create table if not exists proxies (
                  id text primary key,
                  scheme text not null,
                  host text not null,
                  port integer not null,
                  user_enc blob,
                  user_nonce blob,
                  pass_enc blob,
                  pass_nonce blob,
                  shared_slots integer not null default 1,
                  alive integer,
                  latency_ms integer,
                  last_check_at integer,
                  created_at integer not null default (strftime('%s','now'))
                );

                create table if not exists guilds (
                  guild_id text primary key,
                  name text,
                  icon text
                );

                create table if not exists voice_channels (
                  channel_id text primary key,
                  guild_id text not null,
                  name text,
                  favorite integer not null default 0,
                  note text
                );

                create table if not exists inboxes (
                  id text primary key,
                  name text not null,
                  url text not null,
                  domain text,
                  created_at integer not null default (strftime('%s','now'))
                );

                create index if not exists idx_accounts_proxy on accounts(proxy_id);
                create index if not exists idx_voice_channels_guild on voice_channels(guild_id);
                "#,
            )?;
            let _ = c.execute("alter table inboxes add column domain text", []);
            Ok(())
        })
    }

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.with(|c| {
            let mut s = c.prepare("select value from settings where key = ?1")?;
            let v: Option<String> = s.query_row(params![key], |r| r.get(0)).ok();
            Ok(v)
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.with(|c| {
            c.execute(
                "insert into settings(key,value) values(?1,?2)
                 on conflict(key) do update set value=excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
    }
}
