#![allow(incomplete_features)]
#![feature(capture_disjoint_fields, backtrace)]

use color_eyre::eyre::Result;
use config::CONFIG;
use mesagisto_client::MesagistoConfig;
use serenity::{
  client::ClientBuilder, framework::standard::StandardFramework, prelude::GatewayIntents,
};
use smol::future::FutureExt;
use tracing::{info, warn};

use crate::{
  bot::{DcFile, BOT_CLIENT},
  config::Config,
  message::handlers::receive,
};

#[macro_use]
extern crate educe;
#[macro_use]
extern crate automatic_config;
#[macro_use]
extern crate singleton;

mod bot;
mod commands;
mod config;
mod event;
pub mod ext;
mod framework;
mod log;
mod message;
mod net;

#[tokio::main]
async fn main() -> Result<()> {
  if cfg!(feature = "color") {
    color_eyre::install()?;
  } else {
    color_eyre::config::HookBuilder::new()
      .theme(color_eyre::config::Theme::new())
      .install()?;
  }

  self::log::init();
  run().await?;
  Ok(())
}

async fn run() -> Result<()> {
  Config::reload().await?;
  if !CONFIG.enable {
    warn!("Mesagisto-Bot is not enabled and is about to exit the program.");
    warn!("To enable it, please modify the configuration file.");
    warn!("Mesagisto-Bot未被启用, 即将退出程序。");
    warn!("若要启用，请修改配置文件。");
    return Ok(());
  }
  info!(
    "Mesagisto信使正在启动, version: v{}",
    env!("CARGO_PKG_VERSION")
  );
  CONFIG.migrate();
  MesagistoConfig::builder()
    .name("dc")
    .cipher_key(CONFIG.cipher.key.clone())
    .nats_address(CONFIG.nats.address.clone())
    .proxy(if CONFIG.proxy.enable {
      Some(CONFIG.proxy.address.clone())
    } else {
      None
    })
    .photo_url_resolver(|id_pair| {
      async {
        let dc_file: DcFile = serde_cbor::from_slice(&id_pair.1)?;
        Ok(dc_file.to_url())
      }
      .boxed()
    })
    .build()
    .apply()
    .await?;

  let framework = StandardFramework::new()
    .configure(|c| c.prefix("/"))
    .help(&framework::HELP)
    .group(&framework::MESAGISTO_GROUP)
    .normal_message(message::handler::message_hook);

  let http = net::build_http().await;
  let intents = {
    let mut intents = GatewayIntents::all();
    // intents.remove(GatewayIntents::GUILD_PRESENCES);
    intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
    intents.remove(GatewayIntents::DIRECT_MESSAGE_REACTIONS);
    intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);
    intents.remove(GatewayIntents::GUILD_MESSAGE_REACTIONS);
    intents
  };
  let mut client = ClientBuilder::new_with_http(http, intents)
    .event_handler(event::Handler)
    .framework(framework)
    .await?;
  BOT_CLIENT.init(client.cache_and_http.clone());

  let shard_manager = client.shard_manager.clone();
  receive::recover().await?;
  tokio::spawn(async move {
    client.start().await.expect("Client error");
  });

  tokio::signal::ctrl_c().await?;
  shard_manager.lock().await.shutdown_all().await;
  info!("Mesagisto信使 正在关闭");
  info!("正在保存配置文件");
  CONFIG.save().await?;
  Ok(())
}
