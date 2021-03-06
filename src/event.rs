use serenity::{
  async_trait,
  client::{Context, EventHandler},
  model::prelude::Ready,
};
use tracing::info;

pub struct Handler;
#[async_trait]
impl EventHandler for Handler {
  async fn ready(&self, _: Context, ready: Ready) {
    info!("Bot:{} 已连接到Discord服务器!", ready.user.name);
  }
}
