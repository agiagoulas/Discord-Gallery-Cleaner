use serenity::{
    futures::StreamExt,
    http::Http,
    model::{channel::Message, id::ChannelId},
    prelude::*,
};
use std::{env, time::SystemTime};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Log/Output settings
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load config, comment in for non-docker run
    // dotenv::from_filename("./.env").expect("Failed to load .env file");

    let token: String = env::var("DISCORD_TOKEN").expect("Expected token in env.");
    let admin_channel: ChannelId = str_to_channel_id(
        &env::var("ADMIN_CHANNEL_ID").expect("Expected admin channel id in env."),
    );
    let seconds_threshold: u64 = env::var("CLEAN_TIME_SECONDS_THRESHOLD")
        .expect("Expected seconds threshold.")
        .parse()
        .expect("Error parsing to u64");
    let channels_to_clean = env::vars().filter(|c| c.0.starts_with("PURGE_CHANNEL_ID"));
    let allowed_uris_args = env::vars().filter(|c| c.0.starts_with("ALLOWED_URI"));

    let mut allowed_uris = Vec::new();
    for (_, allowed_uri) in allowed_uris_args {
        allowed_uris.push(allowed_uri)
    }

    // connect to api and clean
    info!("Starting clean job.");
    info!("Allowed URIs: {:?}", allowed_uris);
    info!("Clean time seconds threshold: {}", seconds_threshold);
    let intents = GatewayIntents::all();

    let client = Client::builder(token, intents)
        .await
        .expect("Err creating client");
    let ctx = &client.cache_and_http.http;

    for (_, channelid) in channels_to_clean {
        let channel = str_to_channel_id(&channelid);
        let channel_name = channel
            .to_channel(&client.cache_and_http)
            .await
            .unwrap()
            .guild()
            .unwrap()
            .name; // for some reason .name() on ChannelId does not work.

        let (purge_count, count_media_kept) =
            purge_channel(&channel, ctx, &seconds_threshold, &allowed_uris).await;

        let delete_count_msg = format!(
            "Ich habe in #{} {} Nachrichten gelöscht und {} Bilder behalten.",
            channel_name, purge_count, count_media_kept
        );

        admin_channel.say(ctx, &delete_count_msg).await.unwrap();
        info!("{}", &delete_count_msg);
    }
}

async fn purge_channel(
    channel: &ChannelId,
    ctx: &Http,
    seconds_threshold: &u64,
    allowed_uris: &Vec<String>,
) -> (u64, u64) {
    let mut count_deleted = 0;
    let mut count_media_kept = 0;

    let mut messages = channel.messages_iter(&ctx).boxed();

    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(message) => {
                if !message.attachments.is_empty() || linked_image(&message, &allowed_uris) {
                    count_media_kept += 1;
                } else if message_older_than_seconds_threshold(&message, &seconds_threshold) {
                    match message.delete(&ctx).await {
                        Ok(_) => count_deleted += 1,
                        Err(error) => {
                            error!("Error deleting msg: {}. Error: {}", message.id, error)
                        }
                    }
                }
            }
            Err(error) => error!("Error fetching messages: {}", error),
        }
    }

    (count_deleted, count_media_kept)
}

fn linked_image(msg: &Message, allowed_uris: &Vec<String>) -> bool {
    if !msg.content.contains("http") {
        return false;
    }

    for uri in allowed_uris {
        if msg.content.contains(uri) {
            return true;
        }
    }

    false
}

fn message_older_than_seconds_threshold(msg: &Message, seconds_threshold: &u64) -> bool {
    let msg_unix = msg.timestamp.unix_timestamp() as u64;
    let current_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    (current_unix - msg_unix) > *seconds_threshold
}

fn str_to_channel_id(id_as_str: &str) -> ChannelId {
    let channel_id: u64 = id_as_str.parse().expect("Error parsing purge channel id.");
    ChannelId(channel_id)
}
