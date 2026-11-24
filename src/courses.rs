//! Online courses ("教程" / CourseListActivity + CourseDetailActivity).
//!
//! The app fetches `https://app.api.pusuntech.com/TutorialBallWay/get` without
//! authentication. Each course has a name, description, image, a list of videos and
//! `seles`, the drill points its Start button loads into Vary Speed. The API sends no
//! CORS headers, so the page requests it through the `/pusun-api/` proxy configured
//! in Trunk.toml. Images and videos are plain media URLs and load directly.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
const COURSES_URL: &str = "/pusun-api/get";
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

    pub fn image(&self) -> Option<String> {
        let r = self.image_route.as_deref()?.trim();
        if r.is_empty() {
            None
        } else if r.contains("http") {
            Some(r.to_string())
        } else {
            Some(format!("{BASE_URL}{}", r.trim_start_matches('/')))
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_courses() -> Result<Vec<Course>, String> {
    let resp = gloo_net::http::Request::get(COURSES_URL).send().await.map_err(|e| e.to_string())?;
    let is_json = resp.headers().get("content-type").is_some_and(|t| t.contains("json"));
    if !resp.ok() || !is_json {
        return Err(format!(
            "The course list is not reachable here (HTTP {}). Pusun's server does not allow direct browser access, \
             so this page needs the /pusun-api/ proxy that `trunk serve` provides.",
            resp.status()
        ));
    }
    resp.json::<Vec<Course>>().await.map_err(|e| format!("Unexpected course data: {e}"))
}

#[cfg(target_arch = "wasm32")]
/// Course browser. `on_practice` receives the course's drill points.
#[component]
pub fn Courses(on_practice: impl Fn(Vec<u8>) + Copy + Send + Sync + 'static) -> impl IntoView {
    let courses = RwSignal::new(None::<Result<Vec<Course>, String>>);
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
            <h2>"Courses"</h2>
            <Show when=move || courses.get().is_none()>
                <p class="hint">"Training videos and drills from Pusun's servers (text is in Chinese, as in the app)."</p>
                <button on:click=move |_| load() disabled=move || loading.get()>
                    {move || if loading.get() { "Loading…" } else { "Load courses" }}
                </button>
            </Show>
            {move || match courses.get() {
                Some(Err(e)) => view! {
                    <p class="error">{e} " " <button on:click=move |_| load()>"Retry"</button></p>
                }.into_any(),
                Some(Ok(list)) if open.get().is_none() => view! {
                    <div class="courses">
                        {list.into_iter().map(|c| {
                            let pick = c.clone();
                            view! {
                                <button class="course" on:click=move |_| show(pick.clone())>
                                    {c.image().map(|src| view! { <img src=src alt="" loading="lazy" /> })}
                                    <span class="course-name">{c.dao_name.clone()}</span>
                                    <small>{c.info.clone().unwrap_or_default()}</small>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                }.into_any(),
                _ => ().into_any(),
            }}
            {move || open.get().map(|c| {
                let points = c.points();
                let videos = c.tutorial_videos.clone();
                view! {
                    <div class="course-detail">
                        <button on:click=move |_| open.set(None)>"← All courses"</button>
                        <h3>{c.dao_name.clone()}</h3>
                        <video controls playsinline preload="metadata" src=move || playing.get() poster=c.image()></video>
                        <div class="row">
                            {videos.into_iter().map(|v| {
                                let url = v.video.clone();
                                let url2 = url.clone();
                                view! {
                                    <button class="mode" class:selected=move || playing.get() == url
                                        on:click=move |_| playing.set(url2.clone())>{v.video_name}</button>
                                }
                            }).collect_view()}
                        </div>
                        <p>{c.info.clone().unwrap_or_default()}</p>
                        <p class="hint">{format!("Used {} times.", c.use_times)}</p>
                        <button class="primary" disabled=points.is_empty()
                            on:click=move |_| on_practice(points.clone())>"Practice in Vary Speed"</button>
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
        }
        println!("{} courses, {} videos", list.len(), list.iter().map(|c| c.tutorial_videos.len()).sum::<usize>());
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
    }
}
