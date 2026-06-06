use std::sync::mpsc;

use readability_js::{Readability, ReadabilityError};
use scraper::Html;
use tokio::sync::oneshot;

pub struct Payload {
    pub html_doc: String,
    pub sender_chan: oneshot::Sender<Result<String, ReadabilityError>>,
}

pub fn spawn() -> mpsc::Sender<Payload> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let readability = Readability::new().unwrap();
        let mut itr = rx.iter();

        loop {
            let Payload {
                html_doc,
                sender_chan,
            } = itr.next().unwrap();

            let article = match readability.parse(&html_doc) {
                Ok(v) => v,
                Err(e) => {
                    sender_chan.send(Err(e)).unwrap();
                    return;
                }
            };

            let content = Html::parse_fragment(&article.content)
                .root_element()
                .text()
                .collect::<String>();

            sender_chan.send(Ok(content)).unwrap();
        }
    });

    tx
}
