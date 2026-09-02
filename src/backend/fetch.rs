use crate::app::App;
use crate::library::Playable;
use crate::picker::SearchHit;
use crate::backend::Job;

pub fn sync_fetches(app: &mut App) {
    if let Some(i) = app.queue.current {
        ensure(app, i, true);
    }

    for i in wanted_indices(app, app.prefetch) {
        ensure(app, i, false);
    }
}

fn ensure(app: &mut App, i: usize, play: bool) {
    crate::backend::cache::bind_cached(&mut app.queue.items[i]);

    let id = app.queue.items[i].id;
    let Playable::Url(url) = &app.queue.items[i].playable else {
        return;
    };
    if app.in_flight.contains(&id) {
        return;
    }
    app.in_flight.insert(id);
    if play {
        app.awaiting_play = Some(id);
    }
    let _ = app.jobs.send(Job::Fetch {
        id,
        source: app.queue.items[i].source,
        hit: SearchHit {
            title: app.queue.items[i].title.clone(),
            url: url.clone(),
        },
    });
}

fn wanted_indices(app: &App, n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![];
    }
    let len = app.queue.items.len();
    let Some(cur) = app.queue.current else { return vec![] };

    if app.playback.shuffle {
        return (0..len)
            .filter(|j| {
                *j != cur
                    && !app.playback.played.contains(j)
                    && matches!(app.queue.items[*j].playable, Playable::Url(_))
            })
            .take(n)
            .collect();
    }

    let mut out = Vec::new();
    for step in 1..=len {
        if out.len() >= n {
            break;
        }
        let i = cur + step;
        let i = if i < len {
            i
        } else if app.playback.repeat == crate::library::Repeat::All {
            i % len
        } else {
            break;
        };
        if i != cur && matches!(app.queue.items[i].playable, Playable::Url(_)) {
            out.push(i);
        }
    }
    out
}
