//! Travail hors du fil de l'interface : appels HTTP bloquants, delais et minuteries,
//! chacun sur son propre fil systeme pour ne jamais bloquer l'executeur d'iced.

use std::thread;
use std::time::Duration;

use iced::futures::channel::{mpsc, oneshot};
use iced::futures::executor::block_on;
use iced::futures::{SinkExt, Stream};
use iced::{Subscription, Task};

/// Execute `travail` sur un fil a part ; le message est produit a la fin.
pub fn en_fond<T, M>(
    travail: impl FnOnce() -> T + Send + 'static,
    message: impl Fn(T) -> M + Send + 'static,
) -> Task<M>
where
    T: Send + 'static,
    M: Send + 'static,
{
    let (envoi, reception) = oneshot::channel();
    thread::spawn(move || {
        let _ = envoi.send(travail());
    });
    Task::future(reception).then(move |resultat| match resultat {
        Ok(valeur) => Task::done(message(valeur)),
        Err(_) => Task::none(),
    })
}

/// Produit `message` apres `delai`.
pub fn apres<M: Clone + Send + 'static>(delai: Duration, message: M) -> Task<M> {
    en_fond(move || thread::sleep(delai), move |()| message.clone())
}

/// Un evenement toutes les `periode`.
pub fn minuterie(periode: Duration) -> Subscription<()> {
    Subscription::run_with(periode, |periode| tic_tac(*periode))
}

fn tic_tac(periode: Duration) -> impl Stream<Item = ()> {
    let (mut envoi, reception) = mpsc::channel(1);
    thread::spawn(move || {
        loop {
            thread::sleep(periode);
            if block_on(envoi.send(())).is_err() {
                break;
            }
        }
    });
    reception
}
