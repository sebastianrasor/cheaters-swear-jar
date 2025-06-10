mod perspective;

use anyhow::Context as _;
use serenity::all::EmojiId;
use serenity::all::Timestamp;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

const GIFS: &str = include_str!("gifs");

struct SwearCounter;

impl TypeMapKey for SwearCounter {
    type Value = Arc<RwLock<HashMap<u64, u8>>>;
}

struct Bot {
    google_api_key: String,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let Some(guild_id) = msg.guild_id else {
            return;
        };
        if guild_id == 316738004335067139
            && !msg.content.is_empty()
            && GIFS
                .lines()
                .collect::<Vec<&str>>()
                .contains(&msg.content.as_str())
        {
            let _ = msg.react(&ctx, EmojiId::new(1171493411270967447)).await;
        }
        if (include_str!("users")
            .lines()
            .flat_map(|x| x.parse())
            .collect::<Vec<u64>>())
        .contains(&msg.author.id.get())
        {
            let Ok(analyze_comment_response) =
                perspective::analyze_comment(&self.google_api_key, &msg.content).await
            else {
                return;
            };

            let Some(score) = analyze_comment_response.unpack_score_value("PROFANITY") else {
                return;
            };

            if score < 0.5 {
                return;
            }

            let _ = msg.react(&ctx, '‼').await;

            let counter_lock = {
                let data_read = ctx.data.read().await;

                data_read
                    .get::<SwearCounter>()
                    .expect("Expected SwearCounter in TypeMap.")
                    .clone()
            };

            let third_swear: bool;
            {
                let mut counter = counter_lock.write().await;

                let entry = counter.entry(msg.author.id.into()).or_insert(0);
                if *entry == 2 {
                    third_swear = true;
                    *entry = 0;
                } else {
                    third_swear = false;
                    *entry += 1;
                }
            }

            if third_swear {
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

    let google_api_key = secret_store
        .get("GOOGLE_API_KEY")
        .context("'GOOGLE_API_KEY' was not found")?;

    let intents = serenity::model::gateway::GatewayIntents::non_privileged()
        | serenity::model::gateway::GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(discord_token, intents)
        .event_handler(Bot { google_api_key })
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<SwearCounter>(Arc::new(RwLock::new(HashMap::default())));
    }

    Ok(client.into())
}
