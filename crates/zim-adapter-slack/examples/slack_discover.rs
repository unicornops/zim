//! Discover Slack DMs from a headless Tokio consumer.

use std::env;

use zim_adapter_slack::{ConversationKind, SlackClient};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("ZIM_SLACK_TOKEN")?;
    let workspace = env::var("ZIM_SLACK_WORKSPACE")?;
    let slack = SlackClient::connect(&token, &workspace).await?;
    let page = slack.conversations(ConversationKind::Direct, None).await?;
    println!(
        "Found {} accessible direct conversations on the first page.",
        page.items.len()
    );
    Ok(())
}
