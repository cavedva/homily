use homily::general::*;
use homily::keymap::*;
use homily::stringlogger::*;
use homily::ui_crossterm::*;

use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use dirs::home_dir;
use tokio::runtime::Runtime;
use futures::prelude::*;
use futures::future::*;
use futures::{stream, StreamExt};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use log::{info, LevelFilter};
use quick_xml::de::{from_str};
use quick_xml::DeError;
use serde::Deserialize;

#[derive(Copy, Clone, PartialEq)]
enum View {
    Feeds,
    Episodes,
    Headers,
    Downloads,
    Log,
    //Details,
}

impl Default for View {
    fn default() -> Self { View::Feeds }
}

#[derive(Debug, Deserialize, PartialEq)]
struct Rss {
    channel: Channel,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FeedList {
    #[serde(rename = "feed", default)]
    feeds: Vec<Feed>,
}

fn load_rss(filename: PathBuf) -> Result<Rss, ()> {
    fs::read_to_string(&filename).map_err(|_| ()).and_then(
        |r| {
            from_str(&r
                .replace("itunes:title", "itunes-title")
                .replace("& ", "&amp; "))
            .map_err(
                |e| {
                    info!("{:?}", e);
                    match e {
                        DeError::Xml(xe) => info!("xml error: {}", xe),
                        _ => info!("other error"),
                    };
                }
            )
        }
    )
}

fn update_feed(f: &mut Feed, config_path: PathBuf) {
    if let Ok(rss) = load_rss(config_path.join(&f.save_path())) {
        let feed_rc = Rc::new(f.clone());
        f.episodes.things = rss.channel.episodes.into_iter().collect();
        for ep in &mut f.episodes.things {
            (*ep).feed = Some(feed_rc.clone());
        }

        f.episodes.things.sort_by(
            |e1, e2|
            match (e1.pub_date, e2.pub_date) {
                (Some(date1), Some(date2)) => date2.cmp(&date1),
                _ => Ordering::Greater,
            }
        );

        f.check_episodes_downloaded();
        info!("Loaded RSS: {}", f.name);
    } else {
        info!("Failed to load RSS: {}", f.name);
    }
}

fn log_time() {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).expect("");
    info!("{}", t.as_secs() as u128 * 1000 + t.subsec_millis() as u128);
}

fn load_feeds(config_path: PathBuf) -> Vec<Feed> {
    log_time();
    let mut feeds = from_str::<FeedList>(&fs::read_to_string(config_path.join("feeds.xml")).unwrap()).unwrap().feeds;
    log_time();
    feeds.iter_mut().for_each(
        |mut f| update_feed(&mut f, config_path.clone())
    );
    log_time();
    feeds
}

async fn fetch(utx: UnboundedSender<Message>, view_items: Vec<Download>) {
    stream::iter(view_items)
        .map(|view_item| async { download(utx.clone(), view_item).await })
        .buffer_unordered(8)
        .for_each(|_| async move { info!("Feed updated"); } )
        .then(|_| {
            let utx = utx.clone();
            info!("done");       
            utx.unbounded_send(Message::FeedUpdated).unwrap();
            ready::<u32>(0)
        })
        .await;
}

fn main() {
    let mut config_path = home_dir().unwrap();
    config_path.push(".homily");

    let runtime = Runtime::new().unwrap();
    let (utx, mut urx) = unbounded::<Message>();

    log::set_boxed_logger(Box::new(
            StringLogger { utx: Mutex::new(utx.clone()) }
        ))
        .map(|()| log::set_max_level(LevelFilter::Info)).unwrap();

    let mut feeds = ThingList { things: load_feeds(config_path.clone()), ..Default::default() };
    let mut headers = ThingList { ..Default::default() };
    let mut downloads = ThingList { ..Default::default() };
    let mut log_messages = ThingList { ..Default::default() };
    let mut dtlist: ThingList<Thing> = ThingList { things: get_things(&feeds.things), ..Default::default() };
    let mut selected_view = View::Feeds;

    let mut status = Status("".to_string());

    let update_status = |feeds_local: &mut ThingList<Feed>, status: &mut Status, selected_view: &View| {
        match selected_view {
            View::Feeds => status.0 = feeds_local.current().url.clone(),
            View::Episodes => status.0 = feeds_local.current().episodes.current().enclosure.url.clone(),
            _ => {},
        }
    };

    //let term = Term::with_height(TermHeight::Percent(100)).unwrap();
    let mut ta = get_term();
    let (mut width, mut height) = ta.size();
    let mut update_required = true;
    let mut status_update_required = true;
    

    fn switch_view<T>(dtlist: &mut ThingList<Thing>, selected_view: &mut View, new_view: View, thing_list: &ThingList<T>) 
            where T: Styled {
        *selected_view = new_view;
        dtlist.things = get_things(&(thing_list.things));
        dtlist.selected_index = thing_list.selected_index;
    }

    loop {
        if let Some(k) = ta.peek_key() {
            (width, height) = ta.size();

            let mut update_selected_index = |dtlist: &ThingList<Thing>| {
                match selected_view {
                    View::Feeds => feeds.selected_index = dtlist.selected_index,
                    View::Episodes => feeds.current().episodes.selected_index = dtlist.selected_index,
                    View::Headers => headers.selected_index = dtlist.selected_index,
                    View::Downloads => downloads.selected_index = dtlist.selected_index,
                    View::Log => log_messages.selected_index = dtlist.selected_index,
                }
            };

            let mut nav_list = |offset: i32| {
                dtlist.shift_index(offset);
                update_selected_index(&dtlist);
            };

            update_required = true;
            match k {
                KeyMap::Quit => break,
                KeyMap::Up => nav_list(-1),
                KeyMap::Down => nav_list(1),
                KeyMap::Home => {
                    dtlist.selected_index = 0;
                    update_selected_index(&dtlist);
                },
                KeyMap::End => {
                    dtlist.selected_index = dtlist.things.len() - 1;
                    update_selected_index(&dtlist);
                },
                KeyMap::Left | KeyMap::Feeds => switch_view(&mut dtlist, &mut selected_view, View::Feeds, &feeds),
                KeyMap::Downloads => switch_view(&mut dtlist, &mut selected_view, View::Downloads, &downloads),
                KeyMap::Right | KeyMap::Enter => {
                    if selected_view == View::Feeds && feeds.current().episodes.things.len() > 0 {
                        dtlist.things = get_things(&feeds.current().episodes.things);
                        dtlist.selected_index = feeds.current().episodes.selected_index;
                        selected_view = View::Episodes;
                    }
                },
                KeyMap::Refresh => {
                    let mut feed_downloads: Vec<Download> = vec![];
                    for feed in feeds.things.iter() {
                        let mut feed_dl = feed.get_download(Some(&config_path));
                        feed_dl.success_message = Some(Message::FeedDownloaded(feed.name.clone()));
                        feed_downloads.push(feed_dl.clone());
                        downloads.things.push(feed_dl);
                    }
                    runtime.spawn(fetch(utx.clone(), feed_downloads));
                }
                KeyMap::Episodes => switch_view(&mut dtlist, &mut selected_view, View::Episodes, &(feeds.current().episodes)),
                KeyMap::Log => switch_view(&mut dtlist, &mut selected_view, View::Log, &log_messages),
                KeyMap::Headers => { 
                    let header_url: Option<String> = match selected_view {
                        View::Feeds => Some(feeds.current().url.clone()),
                        View::Episodes => Some(feeds.current().episodes.current().enclosure.url.clone()),
                        _ => None,
                    };
                    if let Some(url) = header_url {
                        runtime.spawn(head(utx.clone(), url));
                    }
                    switch_view(&mut dtlist, &mut selected_view, View::Headers, &headers);
                },
                KeyMap::Download => { 
                    let (name, sp, dl) = match selected_view {
                        View::Episodes => (
                            feeds.current().episodes.current().name.clone(),
                            feeds.current().episodes.current().save_path().clone(),
                            feeds.current().episodes.current().get_download(None),
                        ),
                        View::Feeds => (
                            feeds.current().name.clone(),
                            feeds.current().save_path().clone(),
                            feeds.current().get_download(Some(&config_path)),
                        ),
                        _ => continue,
                    };
                    let msg = match selected_view {
                        View::Episodes => Message::EpisodeDownloaded(name),
                        View::Feeds => Message::FeedDownloaded(name),
                        _ => Message::FeedDownloaded(name),
                    };
                    let utx = utx.clone();
                    runtime.spawn(download(utx.clone(), dl.clone())
                    .then(move |_| {
                        info!("Downloaded {}", sp);
                        utx.unbounded_send(msg).unwrap();
                        ready::<u32>(0)
                    }));
                    downloads.things.push(dl);
                },
                KeyMap::Resize(w, h) => {
                    width = w;
                    height = h;
                },
                _ => { update_required = false },
            }
        }

        while let Ok(Some(val)) = urx.try_next() {
            match val {
                Message::Notification(text) => status.0 = format!("{}", text),
                Message::FeedUpdated => {
                    status.0 = format!("Feed updated");
                    feeds.things = load_feeds(config_path.clone());
                },
                Message::FeedDownloaded(feedname) => {
                    status.0 = format!("Downloaded: {}", feedname);
                    if let Some(mut feed) = feeds.things.iter_mut().find(|feed| feed.name == feedname) {
                        update_feed(&mut feed, config_path.clone());
                        info!("Downloaded feed: {}", feed.name);
                    }
                    if let View::Feeds = selected_view {
                        dtlist.things = get_things(&feeds.things);
                    }
                    update_required = true;
                },
                Message::EpisodeDownloaded(text) => {
                    status.0 = format!("Downloaded: {}", text);
                    feeds.current().check_episodes_downloaded();
                    if let View::Episodes = selected_view {
                        dtlist.things = get_things(&feeds.current().episodes.things);
                    }
                    update_required = true;
                },
                Message::Headers(headers_list) => {
                    selected_view = View::Headers;
                    headers.things = headers_list;
                }
                Message::DownloadProgress(id, progress) => {
                    if let Some(dl) = downloads.things.iter_mut().find(|x| x.url == id) {
                        dl.downloaded_bytes = progress;
                    }
                },
                Message::DownloadSize(id, progress) => {
                    if let Some(dl) = downloads.things.iter_mut().find(|x| x.url == id) {
                        dl.total_bytes = progress;
                    }
                },
                Message::LogMessage(text) => log_messages.things.push(text),
            }
            status_update_required = true;
        }

        if update_required {
            update_status(&mut feeds, &mut status, &selected_view);
            ta.update(&dtlist, &status, height, width);
        }

        if update_required || status_update_required {
            ta.update_status(&dtlist, &status, height, width);
        }

        update_required = false;
        status_update_required = false;
    }

    ta.clear();
}