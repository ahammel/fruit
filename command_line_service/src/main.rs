mod repl;

#[tokio::main]
async fn main() {
    repl::run().await;
}
