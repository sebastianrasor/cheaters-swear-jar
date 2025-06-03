use aho_corasick::AhoCorasick;
use anyhow::Context as _;
use serenity::all::Timestamp;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::info;

struct SwearCount;

impl TypeMapKey for SwearCount {
    type Value = Arc<AtomicUsize>;
}

struct Bot {}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.id == 759570155763793950 {
            let ac = AhoCorasick::builder()
                .ascii_case_insensitive(true)
                .build(include_str!("patterns").lines())
                .unwrap();

            if ac.find(&msg.content).is_none() {
                return;
            }

            let _ = msg.react(&ctx, '‼').await;

            let swear_data = {
                let data_read = ctx.data.read().await;
                data_read
                    .get::<SwearCount>()
                    .expect("Expected SwearCount in TypeMap.")
                    .clone()
            };

            let previous_swear_count = swear_data.fetch_add(1, Ordering::SeqCst);
            if previous_swear_count == 2 {
                let _ = swear_data.fetch_sub(3, Ordering::SeqCst);
                let _ = msg
                    .reply(&ctx, "Stop swearing. You need a 5-minute timeout.")
                    .await;
                let Ok(mut member) = msg.member(&ctx).await else {
                    return;
                };
                let Ok(timeout_end_time) =
                    Timestamp::from_unix_timestamp(msg.timestamp.unix_timestamp() + 300)
                else {
                    return;
                };
                let _ = member
                    .disable_communication_until_datetime(&ctx, timeout_end_time)
                    .await;
            }
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let intents = serenity::model::gateway::GatewayIntents::non_privileged()
        | serenity::model::gateway::GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(discord_token, intents)
        .event_handler(Bot {})
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SwearCount>(Arc::new(AtomicUsize::new(0)));
    }

    Ok(client.into())
}
