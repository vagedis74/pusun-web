//! Online courses ("Tutorials": CourseListActivity + CourseDetailActivity).
//!
//! The app fetches `https://app.api.pusuntech.com/TutorialBallWay/get` without
//! authentication. Each course has a name, description, image, a list of videos and
//! `seles`, the drill points its Start button loads into Vary Speed. The API sends no
//! CORS headers, so the page requests it through the `/pusun-api/` proxy configured
//! in Trunk.toml. Static hosts have no proxy: there the page falls back to
//! `courses.json` next to it, a copy of the API response the Pages workflow saves at
//! build time. Images and videos are plain media URLs and load directly.

#[cfg(target_arch = "wasm32")]
use crate::ui::{self, Help, tr};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
const COURSES_URL: &str = "/pusun-api/get";
/// Build-time copy, relative to the page so it works under a sub-path.
#[cfg(target_arch = "wasm32")]
const SNAPSHOT_URL: &str = "courses.json";
/// `MyApplication.BASE_URL`, used for image paths that are not absolute URLs.
const BASE_URL: &str = "https://www.pusuntech.com/";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: i64,
    #[serde(default)]
    pub dao_name: String,
    #[serde(default)]
    pub info: Option<String>,
    #[serde(default)]
    pub image_route: Option<String>,
    #[serde(default)]
    pub use_times: i64,
    #[serde(default)]
    pub seles: Option<String>,
    #[serde(default)]
    pub tutorial_videos: Vec<Video>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    #[serde(default)]
    pub video_name: String,
    #[serde(default)]
    pub video: String,
}

/// Stroke and ball names used in Pusun's course texts.
const TERMS: [(&str, &str); 14] = [
    ("正手击球", "forehand"),
    ("反手击球", "backhand"),
    ("正手削球", "forehand slice"),
    ("反手削球", "backhand slice"),
    ("正手截击", "forehand volley"),
    ("反手截击", "backhand volley"),
    ("放小球", "drop shot"),
    ("近距离击球", "close-range shots"),
    ("中距离击球", "mid-range shots"),
    ("远距离击球", "long-range shots"),
    ("反弹球", "bouncing balls"),
    ("凌空球", "balls out of the air"),
    ("低球", "low balls"),
    ("高球", "high balls"),
];
/// The coach's name (盖奥), romanized.
const COACH: &str = "Gai Ao";

fn term(zh: &str) -> Option<&'static str> {
    TERMS.iter().find(|(z, _)| *z == zh).map(|(_, e)| *e)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// English for a course name, description or video title. Pusun writes them from a
/// few templates; text that matches none of them is returned unchanged.
pub fn translate(zh: &str) -> String {
    let zh = zh.trim();
    let t = |x: &str| term(x).map(capitalize);
    if zh == "普尚机器进阶练习" {
        return "Advanced practice with the Pusun machine".into();
    }
    if let Some(e) = zh.strip_prefix("盖奥-").and_then(t) {
        return format!("{COACH}: {e}");
    }
    if let Some(e) = zh.strip_suffix("是常用的技术之一，课程细分为动作跟练，分解动作和击球进阶练习").and_then(t) {
        return format!("The {} is one of the most used techniques. The course has a follow-along session, a step-by-step breakdown and advanced hitting practice.", e.to_lowercase());
    }
    if let Some(e) = zh.strip_prefix("盖奥进阶课程-实战中").and_then(|x| x.strip_suffix("的处理技巧和方法")).and_then(term) {
        return format!("{COACH} advanced course: techniques for handling {e} in match play.");
    }
    for (suffix, label) in [("跟练", "follow-along"), ("分解动作", "step-by-step breakdown"), ("视频课程", "video lesson")] {
        if let Some(e) = zh.strip_suffix(suffix).and_then(t) {
            return format!("{e}: {label}");
        }
    }
    zh.to_string()
}

#[cfg(target_arch = "wasm32")]
fn localized(zh: &str) -> String {
    if !ui::dutch() {
        return translate(zh);
    }
    let zh = zh.trim();
    let dutch = ["forehand", "backhand", "forehandslice", "backhandslice", "forehandvolley",
        "backhandvolley", "dropshot", "slagen op korte afstand", "slagen op middellange afstand",
        "slagen op lange afstand", "stuiterende ballen", "ballen uit de lucht", "lage ballen", "hoge ballen"];
    let term = |text: &str| TERMS.iter().position(|(key, _)| *key == text).map(|i| dutch[i]);
    if zh == "普尚机器进阶练习" {
        return "Gevorderde oefening met de Pusun-machine".into();
    }
    if let Some(stroke) = zh.strip_prefix("盖奥-").and_then(term) {
        return format!("{COACH}: {}", capitalize(stroke));
    }
    if let Some(stroke) = zh.strip_suffix("是常用的技术之一，课程细分为动作跟练，分解动作和击球进阶练习").and_then(term) {
        return format!("{} is een veelgebruikte techniek. De cursus bevat een meedoenles, een uitleg in stappen en een gevorderde slagoefening.", capitalize(stroke));
    }
    if let Some(stroke) = zh.strip_prefix("盖奥进阶课程-实战中").and_then(|s| s.strip_suffix("的处理技巧和方法")).and_then(term) {
        return format!("Gevorderde cursus van {COACH}: technieken voor {stroke} in wedstrijden.");
    }
    for (suffix, label) in [("跟练", "meedoenles"), ("分解动作", "uitleg in stappen"), ("视频课程", "videoles")] {
        if let Some(stroke) = zh.strip_suffix(suffix).and_then(term) {
            return format!("{}: {label}", capitalize(stroke));
        }
    }
    translate(zh)
}

impl Course {
    /// Vary Speed points (1..=28) from `seles`, as the app parses them.
    pub fn points(&self) -> Vec<u8> {
        self.seles
            .as_deref()
            .unwrap_or("")
            .split(',')
            .filter_map(|x| x.trim().parse::<u8>().ok())
            .filter(|x| (1..=28).contains(x))
            .collect()
    }

    /// Absolute HTTPS routes are used as given; other absolute URLs (plain HTTP, other
    /// schemes, protocol-relative) are dropped; anything else is a path under [`BASE_URL`].
    pub fn image(&self) -> Option<String> {
        let r = self.image_route.as_deref()?.trim();
        if r.is_empty() {
            None
        } else if r.get(..8).is_some_and(|s| s.eq_ignore_ascii_case("https://")) {
            Some(r.to_string())
        } else if r.starts_with("//") || r.contains(':') {
            None
        } else {
            Some(format!("{BASE_URL}{}", r.trim_start_matches('/')))
        }
    }
}

fn subtitle_stem(url: &str) -> Option<&str> {
    let stem = url.split(['?', '#']).next()?.rsplit('/').next()?.strip_suffix(".mp4")?;
    match stem {
        "diqiu" | "fan_gen" | "fangs" | "fanj" | "fanp" | "fantan" | "gaoqiu" | "jief" | "jiez" | "jinju" | "lingkong" | "manfangs" | "manjief" | "manjiez" | "manxiaof" | "manxiaoz" | "pfangs" | "pjief" | "pjiez" | "pxiaof" | "pxiaoz" | "xiaof" | "xiaoz" | "yuanju" | "zheng_gen" | "zhengj" | "zhengp" | "zhongju" => Some(stem),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
/// One subtitle track that follows the app language. With two tracks, Chrome's automatic
/// track selection could switch on the second one as well, showing both at once.
fn SubtitleTracks(stem: String) -> impl IntoView {
    let track = NodeRef::<leptos::html::Track>::new();
    let show = move || {
        if let Some(t) = track.get_untracked().and_then(|el| el.track()) {
            t.set_mode(web_sys::TextTrackMode::Showing);
        }
    };
    let lang = move || if ui::dutch() { "nl" } else { "en" };
    view! {
        <track node_ref=track kind="subtitles" default=true
            src=move || format!("subs/{stem}.{}.vtt", lang())
            srclang=lang
            label=move || if ui::dutch() { "Nederlands" } else { "English" }
            on:load=move |_| show() />
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json(url: &str) -> Result<Vec<Course>, String> {
    let resp = gloo_net::http::Request::get(url).send().await.map_err(|e| e.to_string())?;
    let is_json = resp.headers().get("content-type").is_some_and(|t| t.contains("json"));
    if !resp.ok() || !is_json {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Vec<Course>>().await.map_err(|e| format!("unexpected course data: {e}"))
}

/// Live list through the proxy, else the build-time copy. The flag is true for the copy.
#[cfg(target_arch = "wasm32")]
async fn fetch_courses() -> Result<(Vec<Course>, bool), String> {
    let live = match fetch_json(COURSES_URL).await {
        Ok(list) => return Ok((list, false)),
        Err(e) => e,
    };
    match fetch_json(SNAPSHOT_URL).await {
        Ok(list) => Ok((list, true)),
        Err(copy) => Err(format!(
            "The course list is not reachable here (live: {live}; saved copy: {copy}). Pusun's server does not allow \
             direct browser access, so this page needs the /pusun-api/ proxy that `trunk serve` provides, or a \
             courses.json copy next to it."
        )),
    }
}

#[cfg(target_arch = "wasm32")]
/// Course browser. `on_practice` receives the course's drill points.
#[component]
pub fn Courses(on_practice: impl Fn(Vec<u8>) + Copy + Send + Sync + 'static) -> impl IntoView {
    let courses = RwSignal::new(None::<Result<(Vec<Course>, bool), String>>);
    let loading = RwSignal::new(false);
    let open = RwSignal::new(None::<Course>);
    let playing = RwSignal::new(String::new());

    let load = move || {
        loading.set(true);
        spawn_local(async move {
            courses.set(Some(fetch_courses().await));
            loading.set(false);
        });
    };
    let show = move |c: Course| {
        playing.set(c.tutorial_videos.first().map(|v| v.video.clone()).unwrap_or_default());
        open.set(Some(c));
    };

    view! {
        <section>
            <h2>{move || tr("Courses")}</h2>
            <Show when=move || courses.get().is_none()>
                <p class="hint">{move || tr("Training videos and drills from Pusun's servers. Course texts are translated from Chinese; the videos keep their original audio.")}</p>
                <span class="with-help"><button on:click=move |_| load() disabled=move || loading.get()>
                    {move || if loading.get() { tr("Loading…") } else { tr("Load courses") }}
                </button><Help topic="Load courses" /></span>
            </Show>
            {move || match courses.get() {
                Some(Err(e)) => view! {
                    <p class="error">{e} " " <span class="with-help"><button on:click=move |_| load()>{move || tr("Retry")}</button><Help topic="Retry" /></span></p>
                }.into_any(),
                Some(Ok((list, copy))) if open.get().is_none() => view! {
                    {copy.then(|| view! { <p class="hint">{move || tr("Saved copy of the course list from when this site was built.")}</p> })}
                    <div class="courses">
                        {list.into_iter().map(|c| {
                            let pick = c.clone();
                            let image = c.image();
                            view! {
                                <span class="with-help"><button class="course" on:click=move |_| show(pick.clone())>
                                    {image.map(|src| view! { <img src=src alt="" loading="lazy" /> })}
                                    <span class="course-name">{move || localized(&c.dao_name)}</span>
                                    <small>{move || localized(c.info.as_deref().unwrap_or_default())}</small>
                                </button><Help topic="Course" /></span>
                            }
                        }).collect_view()}
                    </div>
                }.into_any(),
                _ => ().into_any(),
            }}
            {move || open.get().map(|c| {
                let points = c.points();
                let videos = c.tutorial_videos.clone();
                let image = c.image();
                view! {
                    <div class="course-detail">
                        <span class="with-help"><button on:click=move |_| open.set(None)>{move || tr("← All courses")}</button><Help topic="← All courses" /></span>
                        <h3>{move || localized(&c.dao_name)}</h3>
                        {let poster = image;
                        move || {
                            let url = playing.get();
                            let tracks = subtitle_stem(&url).map(|stem| view! { <SubtitleTracks stem=stem.to_owned() /> });
                            view! {
                                <video controls playsinline preload="metadata" src=url poster=poster.clone()>
                                    {tracks}
                                </video>
                            }
                        }}
                        <div class="row">
                            {videos.into_iter().map(|v| {
                                let url = v.video.clone();
                                let url2 = url.clone();
                                view! {
                                    <span class="with-help"><button class="mode" class:selected=move || playing.get() == url
                                        on:click=move |_| playing.set(url2.clone())>{move || localized(&v.video_name)}</button><Help topic="Video" /></span>
                                }
                            }).collect_view()}
                        </div>
                        <p>{move || localized(c.info.as_deref().unwrap_or_default())}</p>
                        <p class="hint">{move || format!("{} {} {}", tr("Used"), c.use_times, tr("times."))}</p>
                        <span class="with-help"><button class="primary" disabled=points.is_empty()
                            on:click=move |_| on_practice(points.clone())>{move || tr("Practice in Vary Speed")}</button><Help topic="Practice in Vary Speed" /></span>
                    </div>
                }
            })}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses a captured API response if `PUSUN_COURSES_JSON` points to one.
    /// Pusun's data is not stored in this repository.
    #[test]
    fn parses_live_response_when_provided() {
        let Ok(path) = std::env::var("PUSUN_COURSES_JSON") else { return };
        let list: Vec<Course> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(!list.is_empty());
        for c in &list {
            assert!(!c.dao_name.is_empty());
            assert!(!c.points().is_empty(), "course {} has no drill points", c.id);
            assert!(c.tutorial_videos.iter().all(|v| v.video.starts_with("http")));
            assert!(c.image().is_some());
            // Every text the page shows must come out in English.
            let texts = [c.dao_name.clone(), c.info.clone().unwrap_or_default()].into_iter().chain(c.tutorial_videos.iter().map(|v| v.video_name.clone()));
            for zh in texts {
                let en = translate(&zh);
                assert!(!en.chars().any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch)), "untranslated: {zh} -> {en}");
            }
        }
        println!("{} courses, {} videos", list.len(), list.iter().map(|c| c.tutorial_videos.len()).sum::<usize>());
    }

    #[test]
    fn translates_course_templates() {
        assert_eq!(translate("盖奥-正手削球"), "Gai Ao: Forehand slice");
        assert_eq!(
            translate("反手截击是常用的技术之一，课程细分为动作跟练，分解动作和击球进阶练习"),
            "The backhand volley is one of the most used techniques. The course has a follow-along session, a step-by-step breakdown and advanced hitting practice."
        );
        assert_eq!(translate("盖奥进阶课程-实战中凌空球的处理技巧和方法"), "Gai Ao advanced course: techniques for handling balls out of the air in match play.");
        assert_eq!(translate("放小球跟练"), "Drop shot: follow-along");
        assert_eq!(translate("正手击球分解动作"), "Forehand: step-by-step breakdown");
        assert_eq!(translate("远距离击球视频课程"), "Long-range shots: video lesson");
        assert_eq!(translate("普尚机器进阶练习"), "Advanced practice with the Pusun machine");
        // Unknown text is shown as is.
        assert_eq!(translate("新课程"), "新课程");
    }

    #[test]
    fn course_fields() {
        let c = Course {
            id: 5,
            dao_name: "x".into(),
            info: None,
            image_route: Some("Upload/a.png".into()),
            use_times: 0,
            seles: Some("19, 3,99,x".into()),
            tutorial_videos: vec![],
        };
        assert_eq!(c.points(), vec![19, 3]);
        assert_eq!(c.image().as_deref(), Some("https://www.pusuntech.com/Upload/a.png"));
        let abs = Course { image_route: Some("https://cdn/x.png".into()), ..c };
        assert_eq!(abs.image().as_deref(), Some("https://cdn/x.png"));
        for bad in ["http://cdn/x.png", "javascript:alert(1)", "//cdn/x.png", "data:image/png;base64,AA"] {
            let c = Course { image_route: Some(bad.into()), ..abs.clone() };
            assert_eq!(c.image(), None, "{bad}");
        }
    }
}

