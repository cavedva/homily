pub mod stringlogger;
pub mod keymap;
pub mod ui_crossterm;
pub mod ui_tuikit;

pub mod general {
    use std::cmp::{min, max};
    use std::default::Default;
    use std::fmt::{Display, Formatter};
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    use chrono::prelude::*;
    use futures_channel::mpsc::UnboundedSender;
    use futures_util::StreamExt;
    use log::info;
    use serde::{Deserialize, Deserializer};
    use tokio::prelude::*;
    use tokio::fs as tokio_fs;
    use crossterm::style::{StyledContent, Stylize};
    use tuikit::attr::{Attr, Effect, Color};
    use tuikit::canvas::Canvas;
    use tuikit::draw::{Draw, DrawResult};
    use tuikit::widget::Widget;

    pub trait Styled: Display {
        fn styles(&self) -> Attr {
            Attr::from(Effect::empty())
        }

        fn crossterm_styles(&self, max_length: usize) -> StyledContent<String> {
            let text = &mut self.to_string();
            let text = match text.char_indices().nth(max_length) {
                None => text,
                Some((idx, _)) => &text[..idx],
            };
            if self.styles() == Attr::from(Effect::BOLD) {
                format!("{}", text).bold().white()
            } else {
                format!("{}", text).stylize()
            }
        }

        fn get_thing(&self) -> Thing {
            Thing {
                text: String::from(format!("{}", &self)),
                styles: self.styles(),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum Message {
        Notification(String),
        LogMessage(String),
        FeedUpdated,
        FeedDownloaded(String),
        EpisodeDownloaded(String),
        DownloadProgress(String, u64),
        DownloadSize(String, u64),
        Headers(Vec<Header>),
    }

    pub trait Downloadable {
        fn url(&self) -> String;
        fn save_path(&self) -> String;

        fn get_download(&self, path_prefix: Option<&PathBuf>) -> Download {
            Download {
                url: self.url(),
                path: match path_prefix {
                    Some(path) => (*path.join(&self.save_path())).to_path_buf(),
                    None => PathBuf::from(&self.save_path()),
                },
                ..Default::default()
            }
        }
    }

    pub struct Thing {
        text: String,
        styles: Attr,
    }

    impl Display for Thing {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", &self.text)
        }
    }

    impl Styled for Thing {
        fn styles(&self) -> Attr {
            self.styles
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Clone)]
    pub struct Enclosure {
        pub url: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    pub struct Channel {
        #[serde(rename = "item", default)]
        pub episodes: Vec<Episode>,
    }

    #[derive(Debug, Deserialize, PartialEq, Clone)]
    pub struct Feed {
        pub name: String,
        pub folder: String,
        #[serde(rename = "save-folder", default)]
        pub save_folder: String,
        #[serde(skip)]
        pub save_path: String,
        pub url: String,
        #[serde(skip)]
        pub episodes: ThingList<Episode>,
    }

    impl Feed {
        pub fn check_episodes_downloaded(&mut self) {
            for ep in self.episodes.things.iter_mut() {
                ep.downloaded = Path::new(&ep.save_path()).exists();
            }
        }
    }

    impl Downloadable for Feed {
        fn url(&self) -> String {
            self.url.clone()
        }

        fn save_path(&self) -> String {
            format!("{}.rss", &self.folder)
        }
    }

    impl Display for Feed {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} ({})", &self.name, &self.folder)
        }
    }

    impl Styled for Feed {
        fn styles(&self) -> Attr {
            if self.episodes.things.len() > 0 &&
                    !self.episodes.things[0].name.to_lowercase().contains("teaser") &&
                    !self.episodes.things[0].downloaded {
                Attr::from(Effect::BOLD)
            } else {
                Attr::from(Effect::empty())
            }
        }
    }

    #[derive(Clone, Debug, Deserialize, PartialEq)]
    pub struct Episode {
        #[serde(rename = "title", default)]
        pub name: String,
        #[serde(rename = "pubDate", deserialize_with = "parse_pub_date")]
        pub pub_date: Option<DateTime<FixedOffset>>,
        pub enclosure: Enclosure,
        #[serde(skip)]
        pub downloaded: bool,
        #[serde(skip)]
        pub feed: Option<Rc<Feed>>,
    }

    impl Episode {
        pub fn filename(&self) -> String {
            format!("{}.mp3", &self.name.replace("/", "_"))
        }
    }

    impl Downloadable for Episode {
        fn url(&self) -> String {
            self.enclosure.url.clone()
        }

        fn save_path(&self) -> String {
            format!("{}/{}", &self.feed.as_ref().unwrap().save_folder, &self.filename())
        }
    }

    impl Display for Episode {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} {}", 
                &self.name, 
                &self.pub_date.map_or(String::from("date unknown"), |dt| dt.to_string()),
                //&self.downloaded,
            )
        }
    }

    impl Styled for Episode {
        fn styles(&self) -> Attr {
            if self.downloaded {
                Attr::from(Effect::BOLD)
            } else {
                Attr::from(Effect::empty())
            }
        }
    }

    impl Styled for String {}

    #[derive(Clone, Debug)]
    pub struct Header {
        pub name: String,
        pub url: String,
    }

    impl Display for Header {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}: {}", &self.name, &self.url)
        }
    }

    impl Styled for Header {}

    #[derive(Clone, Default)]
    pub struct Download {
        pub url: String,
        pub path: PathBuf,
        pub downloaded_bytes: u64,
        pub total_bytes: u64,
        pub success_message: Option<Message>,
    }

    impl Display for Download {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} {}", bytes_pretty(self.downloaded_bytes), &self.path.file_name().unwrap().to_string_lossy())
        }
    }

    impl Styled for Download {}

    #[derive(Clone, Debug)]
    pub struct ThingList<T> {
        pub things: Vec<T>,
        pub selected_index: usize,
    }

    impl<T> ThingList<T> {
        pub fn current(&mut self) -> &mut T {
            &mut self.things[self.selected_index]
        }

        pub fn shift_index(&mut self, offset: i32) {
            self.selected_index = (self.selected_index as i32 + offset)
                .rem_euclid(self.things.len() as i32) as usize
        }
    }

    impl<A, B> PartialEq<ThingList<B>> for ThingList<A> where A: PartialEq<B> {
        fn eq(&self, other: &ThingList<B>) -> bool {
            self.things.eq(&other.things)
        }
    }

    impl<T> Default for ThingList<T> {
        fn default() -> ThingList<T> {
            ThingList { things: vec![] as Vec<T>, selected_index: 0 }
        }
    }

    impl<T> Draw for ThingList<T> where T: Display + Styled {
        fn draw(&self, canvas: &mut dyn Canvas) -> DrawResult<()> {
            if self.things.len() > 0 {
                let (_, height) = canvas.size()?;
                let end = max(self.selected_index + 1, height);
                let end = min(end, self.things.len());
                let start = if end > height { end - height } else { 0 };
                //info!("start: {}, end: {}, height: {}", start, end, height);
                for (i, item) in self.things[start..end].iter().enumerate() {
                    let _ = canvas.print_with_attr(i, 0, 
                        &format!("{}", &item), 
                        if i + start == self.selected_index { BLUE.extend(item.styles()) } else { DEFAULT.extend(item.styles()) }
                    );
                }
            }
            Ok(())
        }
    }

    impl<T> Widget for ThingList<T> where T: Display + Styled {}

    pub struct Status(pub String);

    impl Draw for Status {
        fn draw(&self, canvas: &mut dyn Canvas) -> DrawResult<()> {
            let (_width, height) = canvas.size()?;
            let top = height / 2;
            let _ = canvas.print(top, 0, &self.0);
            Ok(())
        }
    }

    impl Widget for Status {
        fn size_hint(&self) -> (Option<usize>, Option<usize>) {
            (Some(self.0.len()), None)
        }
    }

    const DEFAULT: Attr = Attr{ fg: Color::WHITE, bg: Color::BLACK, effect: Effect::empty() };
    const LIGHT_BLUE: Attr = Attr{ fg: Color::LIGHT_BLUE, bg: Color::BLACK, effect: Effect::empty() };
    const BLUE: Attr = Attr{ fg: Color::BLUE, bg: Color::BLACK, effect: Effect::empty() };

    pub fn get_things<T>(items: &Vec<T>) -> Vec<Thing>
            where T: Styled {
        items.iter().map(|i| i.get_thing()).collect()
    }

    pub fn parse_pub_date<'de, D>(deserializer: D,) -> Result<Option<DateTime<FixedOffset>>, D::Error>
            where D: Deserializer<'de>, {
        if let Ok(s) = String::deserialize(deserializer) {
            Ok(DateTime::parse_from_rfc2822(&s).ok())
        } else {
            Ok(None)
        }
    }

    pub fn bytes_pretty(byte_count: u64) -> String {
        match byte_count {
            bytes if bytes >= (1000 * 1000) => format!("{}.{}M", bytes / (1000 * 1000), bytes.rem_euclid(1000 * 1000) / 100_000),
            bytes if bytes >= 1000 => format!("{}K", bytes / 1000),
            bytes => format!("{}", bytes),
        }
    }

    pub async fn head(utx: UnboundedSender<Message>, url: String) {
        let client = reqwest::Client::new();
        match client.head(&url).send().await {
            Ok(resp) => {
                let headers = resp.headers().iter().map(
                    |(key, value)| Header { 
                        name: String::from(key.as_str()),
                        url: String::from(value.to_str().unwrap()),
                    }
                ).collect();
                utx.unbounded_send(Message::Headers(headers))
                    .unwrap_or_else(|e| info!("err chunk {}", e));
            },
            Err(_) => info!("Couldn't get headers"),
        }
    }

    pub async fn download(utx: UnboundedSender<Message>, dl: Download) {
        info!("downloading {}", &dl.url);
        if let Ok(r) = reqwest::get(&dl.url).await {
            let mut stream = r.bytes_stream();
            info!("url: {}", &dl.url);
            info!("file ok:{}", &dl.path.to_string_lossy());
            let file = tokio_fs::File::create(&dl.path).await;
            let mut total_bytes = 0_u64;
            let mut read_bytes = 0_u64;
            match file {
                Ok(mut f) => {
                    while let Some(y) = stream.next().await {
                        let bytes = y.unwrap();
                        read_bytes += bytes.len() as u64;
                        match f.write_all(&bytes).await {
                            Ok(_) => {
                                total_bytes += bytes.len() as u64;
                                utx.unbounded_send(Message::Notification(format!("bytes:{:>8}{:>8}", bytes_pretty(read_bytes), bytes_pretty(total_bytes))))
                                    .unwrap_or_else(|e| info!("err chunk {}", e));
                                utx.unbounded_send(Message::DownloadProgress(dl.url.clone(), read_bytes))
                                    .unwrap_or_else(|e| info!("err chunk {}", e));
                            },
                            Err(e) => info!("Error writing to {}: {}", &dl.path.to_string_lossy(), e),
                        }
                    }
                    if let Some(msg) = dl.success_message {
                        utx.unbounded_send(msg)
                            .unwrap_or_else(|e| info!("err chunk {}", e));
                    }
                },
                Err(e) => info!("file error: {}", e),
            }
        } else {
            info!("Error downloading: {}", &dl.url);
        }
    }
}