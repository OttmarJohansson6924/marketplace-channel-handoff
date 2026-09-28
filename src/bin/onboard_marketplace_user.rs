use marketplace_channel_handoff::infrai_client::InfraiClient;
use marketplace_channel_handoff::marketplace_handoff::{
    onboard, BuyerUpdate, MarketplaceSignup, OrderHandoff, SellerAsset,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InfraiClient::from_env()?;
    let signup = MarketplaceSignup {
        user_id: "user_42".to_owned(),
        email: "chenhua@changba.com".to_owned(),
        handoff: OrderHandoff {
            seller: SellerAsset {
                title: "Walnut writing desk".to_owned(),
                pickup_note: "Use loading bay B".to_owned(),
            },
            buyer_update: BuyerUpdate {
                order_id: "order_1042".to_owned(),
                status: "ready for pickup".to_owned(),
            },
        },
    };

    let outcome = onboard(&client, &signup).await?;
    println!("onboarding outcome: {outcome:?}");
    Ok(())
}
