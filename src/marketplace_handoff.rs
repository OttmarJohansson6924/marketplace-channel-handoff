use serde::{Deserialize, Serialize};

use crate::infrai_client::{InfraiClient, InfraiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellerAsset {
    pub title: String,
    pub pickup_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyerUpdate {
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHandoff {
    pub seller: SellerAsset,
    pub buyer_update: BuyerUpdate,
}

#[derive(Debug, Clone)]
pub struct MarketplaceSignup {
    pub user_id: String,
    pub email: String,
    pub handoff: OrderHandoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryChannel {
    Email,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnboardingResult {
    Delivered {
        channel: DeliveryChannel,
        message_id: String,
    },
    NoEligibleChannel,
    ConsentMissing,
}

pub fn choose_channel(email_suppressed: bool) -> Option<DeliveryChannel> {
    (!email_suppressed).then_some(DeliveryChannel::Email)
}

pub async fn onboard(
    client: &InfraiClient,
    signup: &MarketplaceSignup,
) -> Result<OnboardingResult, InfraiError> {
    if !client
        .consent_allowed(&signup.user_id, "marketplace_updates")
        .await?
    {
        return Ok(OnboardingResult::ConsentMissing);
    }

    let email_suppressed = client.email_suppressed(&signup.email).await?;

    let Some(channel) = choose_channel(email_suppressed) else {
        return Ok(OnboardingResult::NoEligibleChannel);
    };

    let handoff = format!(
        "Order {} is {}. Seller asset: {}. Pickup: {}",
        signup.handoff.buyer_update.order_id,
        signup.handoff.buyer_update.status,
        signup.handoff.seller.title,
        signup.handoff.seller.pickup_note
    );
    let idempotency_key = format!(
        "welcome:{}:{}",
        signup.user_id, signup.handoff.buyer_update.order_id
    );

    let message_id = client
        .send_email(
            &signup.email,
            "Your marketplace handoff",
            &handoff,
            &idempotency_key,
        )
        .await?;

    Ok(OnboardingResult::Delivered {
        channel,
        message_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_email_when_not_suppressed() {
        assert_eq!(choose_channel(false), Some(DeliveryChannel::Email));
    }

    #[test]
    fn declines_delivery_when_email_is_suppressed() {
        assert_eq!(choose_channel(true), None);
    }
}
