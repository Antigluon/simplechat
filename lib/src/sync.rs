// use futures::channel::mpsc::{self, UnboundedSender};
use futures::{Sink, SinkExt, StreamExt};
use sender_sink::wrappers::UnboundedSenderSink;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub fn funnel_in<S, T>(mut sender: S) -> UnboundedSender<T>
where
    S: Sink<T> + Send + Unpin + 'static,
    T: Send + 'static,
{
    let (tx, rx) = unbounded_channel();
    let mut rx_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    let _task = tokio::spawn(async move {
        // let _ = rx.map(Ok).forward(sender).await;
        while let Some(message) = rx_stream.next().await {
            let _ = sender.send(message).await;
        }
    });
    tx
}

pub fn sink<T>(sender: UnboundedSender<T>) -> UnboundedSenderSink<T> {
    UnboundedSenderSink::from(sender)
}
