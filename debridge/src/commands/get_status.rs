use anyhow::Context;

use crate::api;

pub struct GetStatusArgs {
    pub order_id: String,
}

pub async fn run(args: GetStatusArgs) -> anyhow::Result<()> {
    println!("Querying status for order: {}", args.order_id);

    let resp = api::get_order_status(&args.order_id)
        .await
        .context("Failed to fetch order status")?;

    let status = resp["status"].as_str().unwrap_or("unknown");
    let order_id = resp["orderId"].as_str().unwrap_or(&args.order_id);

    println!("\n=== deBridge DLN Order Status ===");
    println!("Order ID: {}", order_id);
    println!("Status:   {}", status);

    let description = match status {
        "Created" => "Order created — waiting for solver to fulfill on destination chain.",
        "Fulfilled" => "Order fulfilled — destination chain delivery complete.",
        "SentUnlock" => "Solver has initiated unlock on source chain.",
        "ClaimedUnlock" => "Solver has claimed source tokens — settlement complete.",
        "OrderCancelled" => "Order cancelled by user.",
        "SentOrderCancel" => "Cancellation sent — waiting for confirmation.",
        "ClaimedOrderCancel" => "Cancellation complete — source tokens returned.",
        _ => "Unknown status.",
    };
    println!("Info:     {}", description);

    if status == "Fulfilled" || status == "ClaimedUnlock" {
        println!("\nBridge COMPLETE.");
    }

    println!("\n--- Raw JSON ---");
    println!("{}", serde_json::to_string_pretty(&resp).unwrap_or_default());

    Ok(())
}
