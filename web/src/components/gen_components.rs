use crate::components::context::{AppState, UIState};
#[cfg(not(feature = "server_build"))]
use crate::components::downloads_tauri::{
    download_file, remove_episode_from_local_db, update_local_database, update_podcast_database,
};
use crate::components::episodes_layout::SafeHtml;
use crate::components::gen_funcs::format_time;
use crate::requests::pod_req::{
    call_download_episode, call_mark_episode_completed, call_mark_episode_uncompleted,
    call_queue_episode, call_remove_downloaded_episode, call_remove_queued_episode,
    call_remove_saved_episode, call_save_episode, DownloadEpisodeRequest, Episode, EpisodeDownload,
    HistoryEpisode, MarkEpisodeCompletedRequest, QueuePodcastRequest, QueuedEpisode,
    SavePodcastRequest, SavedEpisode,
};
#[cfg(not(feature = "server_build"))]
use crate::requests::pod_req::{
    call_get_episode_metadata, call_get_podcast_details, EpisodeRequest,
};
use crate::requests::search_pods::Episode as SearchNewEpisode;
use crate::requests::search_pods::SearchEpisode;
use crate::requests::search_pods::{call_get_podcast_info, test_connection};
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{console, window, HtmlInputElement, MouseEvent};
use yew::prelude::*;
use yew::Callback;
use yew_router::history::{BrowserHistory, History};
use yewdux::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ErrorMessageProps {
    pub error_message: UseStateHandle<Option<String>>,
}

#[function_component(UseScrollToTop)]
pub fn use_scroll_to_top() -> Html {
    let history = BrowserHistory::new();
    use_effect_with((), move |_| {
        // Create a closure that will be called on history change
        // Create a callback to scroll to the top of the page when the route changes
        let callback = history.listen(move || {
            web_sys::window().unwrap().scroll_to_with_x_and_y(0.0, 0.0);
        });

        // Cleanup function: This will be executed when the component unmounts
        // or the dependencies of the effect change.
        move || drop(callback)
    });

    html! {}
}

#[function_component(ErrorMessage)]
pub fn error_message(props: &ErrorMessageProps) -> Html {
    // Your existing logic here...
    let error_message = use_state(|| None::<String>);
    let (_state, _dispatch) = use_store::<AppState>();

    {
        let error_message = error_message.clone();
        use_effect(move || {
            let window = window().unwrap();
            let document = window.document().unwrap();

            let error_message_clone = error_message.clone();
            let closure = Closure::wrap(Box::new(move |_event: Event| {
                error_message_clone.set(None);
            }) as Box<dyn Fn(_)>);

            if error_message.is_some() {
                document
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
            }

            // Return cleanup function
            move || {
                if error_message.is_some() {
                    document
                        .remove_event_listener_with_callback(
                            "click",
                            closure.as_ref().unchecked_ref(),
                        )
                        .unwrap();
                }
                closure.forget(); // Prevents the closure from being dropped
            }
        });
    }

    if let Some(error) = props.error_message.as_ref() {
        html! {
            <div class="error-snackbar">{ error }</div>
        }
    } else {
        html! {}
    }
}

#[allow(non_camel_case_types)]
#[function_component(Search_nav)]
pub fn search_bar() -> Html {
    let history = BrowserHistory::new();
    let dispatch = Dispatch::<AppState>::global();
    let state: Rc<AppState> = dispatch.get();
    let podcast_value = use_state(|| "".to_string());
    let search_index = use_state(|| "podcast_index".to_string()); // Default to "podcast_index"
    let (_app_state, dispatch) = use_store::<AppState>();

    let history_clone = history.clone();
    let podcast_value_clone = podcast_value.clone();
    let search_index_clone = search_index.clone();
    // State for toggling the dropdown in mobile view
    let mobile_dropdown_open = use_state(|| false);
    let on_submit = {
        Callback::from(move |_: ()| {
            let api_url = state.server_details.as_ref().map(|ud| ud.api_url.clone());
            let history = history_clone.clone();
            let search_value = podcast_value_clone.clone();
            let search_index = search_index_clone.clone();
            let dispatch = dispatch.clone();

            wasm_bindgen_futures::spawn_local(async move {
                dispatch.reduce_mut(|state| state.is_loading = Some(true));
                let cloned_api_url = &api_url.clone();
                match test_connection(&cloned_api_url.clone().unwrap()).await {
                    Ok(_) => {
                        match call_get_podcast_info(&search_value, &api_url.unwrap(), &search_index)
                            .await
                        {
                            Ok(search_results) => {
                                dispatch.reduce_mut(move |state| {
                                    state.search_results = Some(search_results);
                                    state.podcast_added = Some(false);
                                });
                                dispatch.reduce_mut(|state| state.is_loading = Some(false));
                                history.push("/pod_layout"); // Use the route path
                            }
                            Err(_) => {
                                dispatch.reduce_mut(|state| state.is_loading = Some(false));
                            }
                        }
                    }
                    Err(e) => {
                        let error = JsValue::from_str(&format!("Error testing connection: {}", e));
                        console::log_1(&error); // Log the error from test_connection
                        dispatch.reduce_mut(|state| state.is_loading = Some(false));
                    }
                }
            });
        })
    };

    let on_input_change = {
        let podcast_value = podcast_value.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            podcast_value.set(input.value());
        })
    };

    let on_submit_click = {
        let on_submit = on_submit.clone(); // Clone the existing on_submit logic
        Callback::from(move |_: MouseEvent| {
            on_submit.emit(()); // Invoke the existing on_submit logic
        })
    };

    let on_search_click = {
        let on_submit = on_submit.clone();
        let mobile_dropdown_open = mobile_dropdown_open.clone();
        Callback::from(move |_: MouseEvent| {
            if web_sys::window()
                .unwrap()
                .inner_width()
                .unwrap()
                .as_f64()
                .unwrap()
                < 768.0
            {
                mobile_dropdown_open.set(!*mobile_dropdown_open);
            } else {
                on_submit.emit(());
            }
        })
    };

    let prevent_default_submit = {
        let on_submit = on_submit.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default(); // Prevent the default form submission
            on_submit.emit(()); // Emit the on_submit event
        })
    };

    let dropdown_open = use_state(|| false);

    let toggle_dropdown = {
        let dropdown_open = dropdown_open.clone();
        Callback::from(move |_: MouseEvent| {
            dropdown_open.set(!*dropdown_open);
        })
    };

    let on_dropdown_select = {
        let dropdown_open = dropdown_open.clone();
        let search_index = search_index.clone();
        move |category: &str| {
            search_index.set(category.to_string());
            dropdown_open.set(false);
        }
    };

    let on_dropdown_select_itunes = {
        let on_dropdown_select = on_dropdown_select.clone();
        Callback::from(move |_| on_dropdown_select("itunes"))
    };

    let on_dropdown_select_podcast_index = {
        let on_dropdown_select = on_dropdown_select.clone();
        Callback::from(move |_| on_dropdown_select("podcast_index"))
    };

    let search_index_display = match search_index.as_str() {
        "podcast_index" => "Podcast Index",
        "itunes" => "iTunes",
        _ => "Unknown",
    };

    html! {
        <div class="episodes-container w-full search-background"> // Ensure full width and set background color
            <form class="search-bar-container flex justify-end w-full mx-auto border-solid border-b-2 border-color" onsubmit={prevent_default_submit}>
                <div class="relative inline-flex"> // Set a max-width for the search bar content
                    // Dropdown Button
                    <button
                        id="dropdown-button"
                        onclick={toggle_dropdown}
                        class="dropdown-button hidden md:flex md:block flex-shrink-0 z-10 inline-flex items-center py-2.5 px-4 text-sm font-medium text-center border border-r-0 border-gray-300 dark:border-gray-700 rounded-l-lg focus:ring-4 focus:outline-none"
                        type="button"
                    >
                        {format!("{} ", search_index_display)}
                        // SVG icon
                        <svg class="w-2.5 h-2.5 ms-2.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 10 6">
                            <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m1 1 4 4 4-4"/>
                        </svg>
                    </button>
                    // Dropdown Content
                    {
                        if *dropdown_open {
                            html! {
                                <div class="search-dropdown-content-class absolute z-10 divide-y rounded-lg shadow">
                                    <ul class="dropdown-container py-2 text-sm">
                                        <li class="dropdown-option" onclick={on_dropdown_select_itunes.clone()}>{ "iTunes" }</li>
                                        <li class="dropdown-option" onclick={on_dropdown_select_podcast_index.clone()}>{ "Podcast Index" }</li>
                                        // Add more categories as needed
                                    </ul>
                                </div>
                            }
                        } else {
                            html! {}
                        }
                    }

                // Search Input Field
                // <div class="relative w-full">
                    <input
                        type="search"
                        id="search-dropdown"
                        class="search-input block p-2.5 w-full z-20 text-sm rounded-r-lg border hidden md:inline-flex"
                        placeholder="Search"
                        required=true
                        oninput={on_input_change.clone()}
                    />
                </div>
                // Search Button
                <button
                    type="submit"
                    class="search-btn p-2.5 text-sm font-medium rounded-lg border focus:ring-4 focus:outline-none"
                    onclick={on_search_click.clone()}
                >
                        // SVG icon for search button
                        <svg class="w-4 h-4" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 20 20">
                            <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m19 19-4-4m0-7A7 7 0 1 1 1 8a7 7 0 0 1 14 0Z"/>
                        </svg>
                </button>
                {
                    // Mobile dropdown content
                    if *mobile_dropdown_open {
                        html! {
                            <div class="search-drop absolute top-full right-0 z-10 divide-y rounded-lg shadow p-6">
                                // Outline buttons for podcast_index or itunes
                                <div class="inline-flex rounded-md shadow-sm mb-2" role="group">
                                    <button
                                        type="button"
                                        class={format!("px-4 py-2 text-sm font-medium rounded-l-lg search-drop-button {}",
                                            if *search_index == "podcast_index" { "active" } else { "" })}
                                        onclick={on_dropdown_select_podcast_index}
                                    >
                                        {"Podcast Index"}
                                    </button>
                                    <button
                                        type="button"
                                        class={format!("px-4 py-2 text-sm font-medium rounded-r-lg search-drop-button {}",
                                            if *search_index == "itunes" { "active" } else { "" })}
                                        onclick={on_dropdown_select_itunes}
                                    >
                                        {"iTunes"}
                                    </button>
                                </div>
                                // Text field for search
                                <input
                                    type="text"
                                    class="search-input shorter-input block p-2.5 w-full text-sm rounded-lg mb-2"
                                    placeholder="Search"
                                    value={(*podcast_value).clone()}
                                    oninput={on_input_change.clone()}
                                />
                                // Search button
                                <button class="search-btn border-0 no-margin mt-4 font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" onclick={on_submit_click.clone()}>
                                    {"Search"}
                                </button>
                            </div>
                        }
                    }
                    else {
                        html! {}
                    }
                }
            </form>
        </div>
    }
}

#[derive(Properties, Clone)]
pub struct ContextButtonProps {
    pub episode: Box<dyn EpisodeTrait>,
    pub page_type: String,
}

#[function_component(ContextButton)]
pub fn context_button(props: &ContextButtonProps) -> Html {
    let dropdown_open = use_state(|| false);
    let (post_state, post_dispatch) = use_store::<AppState>();
    let (_audio_state, audio_dispatch) = use_store::<UIState>();
    let api_key = post_state
        .auth_details
        .as_ref()
        .map(|ud| ud.api_key.clone());
    let user_id = post_state.user_details.as_ref().map(|ud| ud.UserID.clone());
    let server_name = post_state
        .auth_details
        .as_ref()
        .map(|ud| ud.server_name.clone());
    let dropdown_ref = NodeRef::default();

    let toggle_dropdown = {
        let dropdown_open = dropdown_open.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation(); // Stop the event from propagating further
            dropdown_open.set(!*dropdown_open);
        })
    };

    {
        let dropdown_open = dropdown_open.clone(); // Clone for use in the effect hook
        let dropdown_ref = dropdown_ref.clone(); // Clone for use in the effect hook

        // Use this cloned state specifically for checking within the closure
        let dropdown_state_for_closure = dropdown_open.clone();

        use_effect_with(dropdown_open.clone(), move |_| {
            let document = web_sys::window().unwrap().document().unwrap();
            let dropdown_ref_clone = dropdown_ref.clone(); // Clone again to move into the closure

            let click_handler_closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                if let Some(target) = event.target() {
                    if let Some(dropdown_element) =
                        dropdown_ref_clone.cast::<web_sys::HtmlElement>()
                    {
                        if let Ok(node) = target.dyn_into::<web_sys::Node>() {
                            if !dropdown_element.contains(Some(&node)) {
                                dropdown_state_for_closure.set(false);
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            // Only add the event listener if the dropdown is open to avoid unnecessary listeners
            if *dropdown_open {
                document
                    .add_event_listener_with_callback(
                        "click",
                        click_handler_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }

            // Cleanup function
            move || {
                // Always remove the event listener to avoid memory leaks
                document
                    .remove_event_listener_with_callback(
                        "click",
                        click_handler_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }
        });
    }

    let queue_api_key = api_key.clone();
    let queue_server_name = server_name.clone();
    let queue_post = audio_dispatch.clone();
    // let server_name = server_name.clone();
    let on_add_to_queue = {
        let episode = props.episode.clone();
        Callback::from(move |_| {
            let server_name_copy = queue_server_name.clone();
            let api_key_copy = queue_api_key.clone();
            let queue_post = queue_post.clone();
            let request = QueuePodcastRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_queue_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("Episode added to Queue!")));
                match call_queue_episode(&server_name.unwrap(), &api_key.flatten(), &request).await
                {
                    Ok(success_message) => {
                        queue_post.reduce_mut(|state| {
                            state.info_message = Option::from(format!("{}", success_message))
                        });
                    }
                    Err(e) => {
                        queue_post.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let remove_queue_api_key = api_key.clone();
    let remove_queue_server_name = server_name.clone();
    let remove_queue_post = audio_dispatch.clone();
    let dispatch_clone = post_dispatch.clone();
    // let server_name = server_name.clone();
    let on_remove_queued_episode = {
        let episode = props.episode.clone();
        let episode_id = props.episode.get_episode_id();
        Callback::from(move |_| {
            let post_dispatch = dispatch_clone.clone();
            let server_name_copy = remove_queue_server_name.clone();
            let api_key_copy = remove_queue_api_key.clone();
            let queue_post = remove_queue_post.clone();
            let request = QueuePodcastRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_queue_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("Episode added to Queue!")));
                match call_remove_queued_episode(
                    &server_name.unwrap(),
                    &api_key.flatten(),
                    &request,
                )
                .await
                {
                    Ok(success_message) => {
                        // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("{}", success_message)));
                        post_dispatch.reduce_mut(|state| {
                            // Here, you should remove the episode from the queued_episodes
                            if let Some(ref mut queued_episodes) = state.queued_episodes {
                                queued_episodes
                                    .episodes
                                    .retain(|ep| ep.get_episode_id() != episode_id);
                            }
                            // Optionally, you can update the info_message with success message
                            state.info_message = Some(format!("{}", success_message).to_string());
                        });
                    }
                    Err(e) => {
                        queue_post.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let saved_api_key = api_key.clone();
    let saved_server_name = server_name.clone();
    let save_post = audio_dispatch.clone();
    let on_save_episode = {
        let episode = props.episode.clone();
        Callback::from(move |_| {
            let server_name_copy = saved_server_name.clone();
            let api_key_copy = saved_api_key.clone();
            let post_state = save_post.clone();
            let request = SavePodcastRequest {
                episode_id: episode.get_episode_id(), // changed from episode_title
                user_id: user_id.unwrap(),            // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let return_mes = call_save_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // post_state.reduce_mut(|state| state.info_message = Option::from(format!("Episode saved successfully")));
                match call_save_episode(&server_name.unwrap(), &api_key.flatten(), &request).await {
                    Ok(success_message) => {
                        post_state.reduce_mut(|state| {
                            state.info_message = Option::from(format!("{}", success_message))
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let remove_saved_api_key = api_key.clone();
    let remove_saved_server_name = server_name.clone();
    let remove_save_post = audio_dispatch.clone();
    let dispatch_clone = post_dispatch.clone();
    let on_remove_saved_episode = {
        let episode = props.episode.clone();
        let episode_id = props.episode.get_episode_id();
        Callback::from(move |_| {
            let post_dispatch = dispatch_clone.clone();
            let server_name_copy = remove_saved_server_name.clone();
            let api_key_copy = remove_saved_api_key.clone();
            let post_state = remove_save_post.clone();
            let request = SavePodcastRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(),
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                match call_remove_saved_episode(&server_name.unwrap(), &api_key.flatten(), &request)
                    .await
                {
                    Ok(success_message) => {
                        // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("{}", success_message)));
                        post_dispatch.reduce_mut(|state| {
                            // Here, you should remove the episode from the saved_episodes
                            if let Some(ref mut saved_episodes) = state.saved_episodes {
                                saved_episodes
                                    .episodes
                                    .retain(|ep| ep.get_episode_id() != episode_id);
                            }
                            // Optionally, you can update the info_message with success message
                            state.info_message = Some(format!("{}", success_message).to_string());
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let download_api_key = api_key.clone();
    let download_server_name = server_name.clone();
    let download_post = audio_dispatch.clone();
    let on_download_episode = {
        let episode = props.episode.clone();
        Callback::from(move |_| {
            let post_state = download_post.clone();
            let server_name_copy = download_server_name.clone();
            let api_key_copy = download_api_key.clone();
            let request = DownloadEpisodeRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_download_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // post_state.reduce_mut(|state| state.info_message = Option::from(format!("Episode now downloading!")));
                match call_download_episode(&server_name.unwrap(), &api_key.flatten(), &request)
                    .await
                {
                    Ok(success_message) => {
                        post_state.reduce_mut(|state| {
                            state.info_message = Option::from(format!("{}", success_message))
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };
    #[cfg(not(feature = "server_build"))]
    let on_local_episode_download = {
        let episode = props.episode.clone();
        let download_local_post = audio_dispatch.clone();
        let server_name_copy = server_name.clone();
        let api_key_copy = api_key.clone();
        let user_id_copy = user_id.clone();

        Callback::from(move |_| {
            let post_state = download_local_post.clone();
            let episode_id = episode.get_episode_id();
            let request = EpisodeRequest {
                episode_id,
                user_id: user_id_copy.unwrap(),
            };
            let server_name = server_name_copy.clone().unwrap();
            let ep_api_key = api_key_copy.clone().flatten();
            let api_key = api_key_copy.clone().flatten();

            let future = async move {
                match call_get_episode_metadata(&server_name, ep_api_key, &request).await {
                    Ok(episode_info) => {
                        let audio_url = episode_info.episodeurl.clone();
                        let artwork_url = episode_info.episodeartwork.clone();
                        let podcast_id = episode_info.podcastid.clone();
                        let filename = format!("episode_{}.mp3", episode_id);
                        let artwork_filename = format!("artwork_{}.jpg", episode_id);
                        post_state.reduce_mut(|state| {
                            state.info_message = Some(format!("Episode download queued!"))
                        });
                        // Download audio
                        match download_file(audio_url, filename.clone()).await {
                            Ok(_) => {}
                            Err(e) => {
                                post_state.reduce_mut(|state| {
                                    state.error_message =
                                        Some(format!("Failed to download episode audio: {:?}", e))
                                });
                            }
                        }

                        // Download artwork
                        if let Err(e) = download_file(artwork_url, artwork_filename.clone()).await {
                            post_state.reduce_mut(|state| {
                                state.error_message =
                                    Some(format!("Failed to download episode artwork: {:?}", e))
                            });
                        }

                        // Update local JSON database
                        if let Err(e) = update_local_database(episode_info).await {
                            post_state.reduce_mut(|state| {
                                state.error_message =
                                    Some(format!("Failed to update local database: {:?}", e))
                            });
                        }

                        // Fetch and update local podcast metadata
                        match call_get_podcast_details(
                            &server_name,
                            &api_key.unwrap(),
                            user_id_copy.unwrap(),
                            &podcast_id,
                        )
                        .await
                        {
                            Ok(podcast_details) => {
                                if let Err(e) = update_podcast_database(podcast_details).await {
                                    post_state.reduce_mut(|state| {
                                        state.error_message = Some(format!(
                                            "Failed to update podcast database: {:?}",
                                            e
                                        ))
                                    });
                                }
                            }
                            Err(e) => {
                                post_state.reduce_mut(|state| {
                                    state.error_message =
                                        Some(format!("Failed to fetch podcast metadata: {:?}", e))
                                });
                            }
                        }
                    }
                    Err(e) => {
                        post_state
                            .reduce_mut(|state| state.error_message = Some(format!("s {:?}", e)));
                    }
                }
            };

            wasm_bindgen_futures::spawn_local(future);
        })
    };

    #[cfg(not(feature = "server_build"))]
    let on_remove_locally_downloaded_episode = {
        let episode = props.episode.clone();
        let download_local_post = audio_dispatch.clone();

        Callback::from(move |_| {
            let post_state = download_local_post.clone();
            let episode_id = episode.get_episode_id();

            let future = async move {
                let filename = format!("episode_{}.mp3", episode_id);

                // Download audio
                match remove_episode_from_local_db(episode_id).await {
                    Ok(_) => {
                        post_state.reduce_mut(|state| {
                            state.info_message =
                                Some(format!("Episode {} downloaded locally!", filename));
                            if let Some(increment) = state.local_download_increment.as_mut() {
                                *increment += 1;
                            } else {
                                state.local_download_increment = Some(1);
                            }
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message =
                                Some(format!("Failed to download episode audio: {:?}", e))
                        });
                    }
                }
            };

            wasm_bindgen_futures::spawn_local(future);
        })
    };

    let remove_download_api_key = api_key.clone();
    let remove_download_server_name = server_name.clone();
    let remove_download_post = audio_dispatch.clone();
    let dispatch_clone = post_dispatch.clone();
    let on_remove_downloaded_episode = {
        let episode = props.episode.clone();
        let episode_id = props.episode.get_episode_id();
        Callback::from(move |_| {
            let post_dispatch = dispatch_clone.clone();
            let post_state = remove_download_post.clone();
            let server_name_copy = remove_download_server_name.clone();
            let api_key_copy = remove_download_api_key.clone();
            let request = DownloadEpisodeRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_download_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // post_state.reduce_mut(|state| state.info_message = Option::from(format!("Episode now downloading!")));
                match call_remove_downloaded_episode(
                    &server_name.unwrap(),
                    &api_key.flatten(),
                    &request,
                )
                .await
                {
                    Ok(success_message) => {
                        // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("{}", success_message)));
                        post_dispatch.reduce_mut(|state| {
                            // Here, you should remove the episode from the downloaded_episodes
                            if let Some(ref mut downloaded_episodes) = state.downloaded_episodes {
                                downloaded_episodes
                                    .episodes
                                    .retain(|ep| ep.get_episode_id() != episode_id);
                            }
                            // Optionally, you can update the info_message with success message
                            state.info_message = Some(format!("{}", success_message).to_string());
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let uncomplete_api_key = api_key.clone();
    let uncomplete_server_name = server_name.clone();
    let uncomplete_download_post = audio_dispatch.clone();
    let uncomplete_dispatch_clone = post_dispatch.clone();
    let on_uncomplete_episode = {
        let episode = props.episode.clone();
        let episode_id = props.episode.get_episode_id();
        Callback::from(move |_| {
            let post_dispatch = uncomplete_dispatch_clone.clone();
            let post_state = uncomplete_download_post.clone();
            let server_name_copy = uncomplete_server_name.clone();
            let api_key_copy = uncomplete_api_key.clone();
            let request = MarkEpisodeCompletedRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_download_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // post_state.reduce_mut(|state| state.info_message = Option::from(format!("Episode now downloading!")));
                match call_mark_episode_uncompleted(
                    &server_name.unwrap(),
                    &api_key.flatten(),
                    &request,
                )
                .await
                {
                    Ok(success_message) => {
                        // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("{}", success_message)));
                        post_dispatch.reduce_mut(|state| {
                            if let Some(completed_episodes) = state.completed_episodes.as_mut() {
                                if let Some(pos) =
                                    completed_episodes.iter().position(|&id| id == episode_id)
                                {
                                    completed_episodes.remove(pos);
                                } else {
                                    completed_episodes.push(episode_id);
                                }
                            } else {
                                state.completed_episodes = Some(vec![episode_id]);
                            }
                            state.info_message = Some(format!("{}", success_message));
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let complete_api_key = api_key.clone();
    let complete_server_name = server_name.clone();
    let complete_download_post = audio_dispatch.clone();
    let dispatch_clone = post_dispatch.clone();
    let on_complete_episode = {
        let episode = props.episode.clone();
        let episode_id = props.episode.get_episode_id();
        Callback::from(move |_| {
            let post_dispatch = dispatch_clone.clone();
            let post_state = complete_download_post.clone();
            let server_name_copy = complete_server_name.clone();
            let api_key_copy = complete_api_key.clone();
            let request = MarkEpisodeCompletedRequest {
                episode_id: episode.get_episode_id(),
                user_id: user_id.unwrap(), // replace with the actual user ID
            };
            let server_name = server_name_copy; // replace with the actual server name
            let api_key = api_key_copy; // replace with the actual API key
            let future = async move {
                // let _ = call_download_episode(&server_name.unwrap(), &api_key.flatten(), &request).await;
                // post_state.reduce_mut(|state| state.info_message = Option::from(format!("Episode now downloading!")));
                match call_mark_episode_completed(
                    &server_name.unwrap(),
                    &api_key.flatten(),
                    &request,
                )
                .await
                {
                    Ok(success_message) => {
                        // queue_post.reduce_mut(|state| state.info_message = Option::from(format!("{}", success_message)));
                        post_dispatch.reduce_mut(|state| {
                            if let Some(completed_episodes) = state.completed_episodes.as_mut() {
                                if let Some(pos) =
                                    completed_episodes.iter().position(|&id| id == episode_id)
                                {
                                    completed_episodes.remove(pos);
                                } else {
                                    completed_episodes.push(episode_id);
                                }
                            } else {
                                state.completed_episodes = Some(vec![episode_id]);
                            }
                            state.info_message = Some(format!("{}", success_message));
                        });
                    }
                    Err(e) => {
                        post_state.reduce_mut(|state| {
                            state.error_message = Option::from(format!("{}", e))
                        });
                        // Handle error, e.g., display the error message
                    }
                }
            };
            wasm_bindgen_futures::spawn_local(future);
            // dropdown_open.set(false);
        })
    };

    let check_episode_id = props.episode.get_episode_id();
    let is_completed = post_state
        .completed_episodes
        .as_ref()
        .unwrap_or(&vec![])
        .contains(&check_episode_id);

    let on_toggle_complete = {
        let on_complete_episode = on_complete_episode.clone();
        let on_uncomplete_episode = on_uncomplete_episode.clone();
        let is_completed = is_completed.clone();

        Callback::from(move |_| {
            if is_completed {
                on_uncomplete_episode.emit(());
            } else {
                on_complete_episode.emit(());
            }
        })
    };

    #[cfg(feature = "server_build")]
    let download_button = html! {
        <li class="dropdown-option" onclick={on_download_episode.clone()}>{ "Download Episode" }</li>
    };

    #[cfg(not(feature = "server_build"))]
    let download_button = html! {
        <>
            <li class="dropdown-option" onclick={on_download_episode.clone()}>{ "Server Download" }</li>
            <li class="dropdown-option" onclick={on_local_episode_download.clone()}>{ "Local Download" }</li>
        </>
    };

    #[cfg(not(feature = "server_build"))]
    let local_download_options = html! {
        <>
            <li class="dropdown-option" onclick={on_add_to_queue.clone()}>{ "Queue Episode" }</li>
            <li class="dropdown-option" onclick={on_save_episode.clone()}>{ "Save Episode" }</li>
            <li class="dropdown-option" onclick={on_remove_locally_downloaded_episode.clone()}>{ "Remove Downloaded Episode" }</li>
            <li class="dropdown-option" onclick={on_toggle_complete.clone()}>{ if is_completed { "Mark Episode Incomplete" } else { "Mark Episode Complete" } }</li>
        </>
    };

    #[cfg(feature = "server_build")]
    let local_download_options = html! {};

    let action_buttons = match props.page_type.as_str() {
        "saved" => html! {
            <>
                <li class="dropdown-option" onclick={on_add_to_queue.clone()}>{ "Queue Episode" }</li>
                <li class="dropdown-option" onclick={on_remove_saved_episode.clone()}>{ "Remove Saved Episode" }</li>
                {
                    download_button.clone()
                }
                <li class="dropdown-option" onclick={on_toggle_complete.clone()}>{ if is_completed { "Mark Episode Incomplete" } else { "Mark Episode Complete" } }</li>
            </>
        },
        "queue" => html! {
            <>
                <li class="dropdown-option" onclick={on_save_episode.clone()}>{ "Save Episode" }</li>
                <li class="dropdown-option" onclick={on_remove_queued_episode.clone()}>{ "Remove from Queue" }</li>
                {
                    download_button.clone()
                }
                <li class="dropdown-option" onclick={on_toggle_complete.clone()}>{ if is_completed { "Mark Episode Incomplete" } else { "Mark Episode Complete" } }</li>
            </>
        },
        "downloads" => html! {
            <>
                <li class="dropdown-option" onclick={on_add_to_queue.clone()}>{ "Queue Episode" }</li>
                <li class="dropdown-option" onclick={on_save_episode.clone()}>{ "Save Episode" }</li>
                <li class="dropdown-option" onclick={on_remove_downloaded_episode.clone()}>{ "Remove Downloaded Episode" }</li>
                <li class="dropdown-option" onclick={on_toggle_complete.clone()}>{ if is_completed { "Mark Episode Incomplete" } else { "Mark Episode Complete" } }</li>
            </>
        },
        "local_downloads" => html! {
            local_download_options
        },
        // Add more page types and their respective button sets as needed
        _ => html! {
            // Default set of buttons for other page types
            <>
                <li class="dropdown-option" onclick={on_add_to_queue.clone()}>{ "Queue Episode" }</li>
                <li class="dropdown-option" onclick={on_save_episode.clone()}>{ "Save Episode" }</li>
                {
                    download_button.clone()
                }
                <li class="dropdown-option" onclick={on_toggle_complete.clone()}>{ if is_completed { "Mark Episode Incomplete" } else { "Mark Episode Complete" } }</li>
            </>
        },
    };

    html! {
        <>
            <div class="relative show-on-large">
            <button
                id="dropdown-button"
                onclick={toggle_dropdown.clone()}
                class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
            >
                <span class="material-icons large-material-icons md:text-6xl text-4xl">{"more_vert"}</span>
            </button>
            // Dropdown Content
            {
                if *dropdown_open {
                    html! {
                        <div ref={dropdown_ref.clone()} class="dropdown-content-class border border-solid absolute z-10 divide-y rounded-lg shadow w-48">
                            <ul class="dropdown-container py-2 text-sm text-gray-700">
                                { action_buttons }
                            </ul>
                        </div>
                    }
                } else {
                    html! {}
                }
            }
        </div>
        </>
    }
}

pub fn empty_message(header: &str, paragraph: &str) -> Html {
    html! {
        <div class="empty-episodes-container">
            <img src="static/assets/favicon.png" alt="Logo" class="logo"/>
            <h1 class="page-paragraphs">{ header }</h1>
            <p class="page-paragraphs">{ paragraph }</p>
        </div>
    }
}

pub trait EpisodeTrait {
    fn get_episode_artwork(&self) -> String;
    fn get_episode_title(&self) -> String;
    fn get_episode_id(&self) -> i32;
    fn clone_box(&self) -> Box<dyn EpisodeTrait>;
    // fn eq(&self, other: &dyn EpisodeTrait) -> bool;
    fn as_any(&self) -> &dyn Any;
}

impl PartialEq for ContextButtonProps {
    fn eq(&self, _other: &Self) -> bool {
        if let Some(other) = self.episode.as_any().downcast_ref::<Episode>() {
            if let Some(self_episode) = self.episode.as_any().downcast_ref::<Episode>() {
                return self_episode == other;
            }
        }

        if let Some(other) = self.episode.as_any().downcast_ref::<QueuedEpisode>() {
            if let Some(self_episode) = self.episode.as_any().downcast_ref::<QueuedEpisode>() {
                return self_episode == other;
            }
        }

        false
    }
}

impl Clone for Box<dyn EpisodeTrait> {
    fn clone(&self) -> Box<dyn EpisodeTrait> {
        self.clone_box()
    }
}

impl EpisodeTrait for Episode {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    // Implement other methods
}

impl EpisodeTrait for QueuedEpisode {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EpisodeTrait for SavedEpisode {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EpisodeTrait for HistoryEpisode {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EpisodeTrait for EpisodeDownload {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EpisodeTrait for SearchEpisode {
    fn get_episode_artwork(&self) -> String {
        self.episodeartwork.clone()
    }

    fn get_episode_title(&self) -> String {
        self.episodetitle.clone()
    }

    fn get_episode_id(&self) -> i32 {
        self.episodeid.clone()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EpisodeTrait for SearchNewEpisode {
    fn get_episode_artwork(&self) -> String {
        self.artwork.clone().unwrap()
    }

    fn get_episode_title(&self) -> String {
        self.title.clone().unwrap()
    }

    fn get_episode_id(&self) -> i32 {
        self.episode_id.clone().unwrap()
    }

    fn clone_box(&self) -> Box<dyn EpisodeTrait> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Implement other methods
pub fn on_shownotes_click(
    history: BrowserHistory,
    dispatch: Dispatch<AppState>,
    episode_id: Option<i32>,
    shownotes_episode_url: Option<String>,
    episode_audio_url: Option<String>,
    podcast_title: Option<String>,
    _db_added: bool,
) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| {
        let dispatch_clone = dispatch.clone();
        let history_clone = history.clone();
        let shownotes_episode_url_call = shownotes_episode_url.clone();
        let episode_audio_url = episode_audio_url.clone();
        let podcast_title = podcast_title.clone();
        wasm_bindgen_futures::spawn_local(async move {
            dispatch_clone.reduce_mut(move |state| {
                state.selected_episode_id = episode_id;
                state.selected_episode_url = shownotes_episode_url_call.clone();
                state.selected_episode_audio_url = episode_audio_url;
                state.selected_podcast_title = podcast_title;
            });
            history_clone.push("/episode"); // Use the route path
        });
    })
}

pub fn episode_item(
    episode: Box<dyn EpisodeTrait>,
    description: String,
    is_expanded: bool,
    format_release: &str,
    on_play_click: Callback<MouseEvent>,
    on_shownotes_click: Callback<MouseEvent>,
    toggle_expanded: Callback<MouseEvent>,
    episode_duration: i32,
    listen_duration: Option<i32>,
    page_type: &str,
    on_checkbox_change: Callback<i32>,
    is_delete_mode: bool, // Add this line
    ep_url: String,
    completed: bool,
) -> Html {
    let span_duration = listen_duration.clone();
    let span_episode = episode_duration.clone();
    let formatted_duration = format_time(span_episode as f64);
    let formatted_listen_duration = span_duration.map(|ld| format_time(ld as f64));
    // Calculate the percentage of the episode that has been listened to
    let listen_duration_percentage = listen_duration.map_or(0.0, |ld| {
        if episode_duration > 0 {
            (ld as f64 / episode_duration as f64) * 100.0
        } else {
            0.0 // Avoid division by zero
        }
    });
    let checkbox_ep = episode.get_episode_id();
    let should_show_buttons = !ep_url.is_empty();

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggleDescription(guid: &str, expanded: bool);
    }
    let description_class = if is_expanded {
        "desc-expanded".to_string()
    } else {
        "desc-collapsed".to_string()
    };
    html! {
        <div>
            <div class="item-container border-solid border flex items-start mb-4 shadow-md rounded-lg h-full">
                {if is_delete_mode {
                    html! {
                        <input type="checkbox" class="form-checkbox h-5 w-5 text-blue-600"
                            onchange={on_checkbox_change.reform(move |_| checkbox_ep)} /> // Modify this line
                    }
                } else {
                    html! {}
                }}
                <div class="flex flex-col w-auto object-cover pl-4">
                    <img
                        src={episode.get_episode_artwork()}
                        alt={format!("Cover for {}", episode.get_episode_title())}
                        class="episode-image"
                    />
                </div>
                <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                    <div class="flex items-center space-x-2 cursor-pointer" onclick={on_shownotes_click}>
                        <p class="item_container-text episode-title font-semibold">
                            { episode.get_episode_title() }
                        </p>
                        {
                            if completed.clone() {
                                html! {
                                    <span class="material-bonus-color item_container-text material-icons text-md text-green-500">{"check_circle"}</span>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                    <hr class="my-2 border-t hidden md:block"/>
                    {
                        html! {
                            <div class="item-description-text hidden md:block">
                                <div class={format!("item_container-text episode-description-container {}", description_class)}>
                                    <SafeHtml html={description} />
                                </div>
                                <a class="link hover:underline cursor-pointer mt-4" onclick={toggle_expanded}>
                                    { if is_expanded { "See Less" } else { "See More" } }
                                </a>
                            </div>
                        }
                    }
                    <span class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2" style="flex-grow: 0; flex-shrink: 0; width: auto;">
                        <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                            <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                        </svg>
                        { format_release }
                    </span>
                    {
                        if completed {
                            html! {
                                <div class="flex items-center space-x-2">
                                    <span class="item_container-text">{ formatted_duration }</span>
                                    <span class="item_container-text">{ "-  Completed" }</span>
                                </div>
                            }
                        } else {
                            if formatted_listen_duration.is_some() {
                                html! {
                                    <div class="flex items-center space-x-2">
                                        <span class="item_container-text">{ formatted_listen_duration.clone() }</span>
                                        <div class="progress-bar-container">
                                            <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                        </div>
                                        <span class="item_container-text">{ formatted_duration }</span>
                                    </div>
                                }
                            } else {
                                html! {
                                    <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                }
                            }
                        }
                    }
                </div>
                {
                    html! {
                        <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;">
                            if should_show_buttons {
                                <button
                                    class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                    onclick={on_play_click}
                                >
                                    <span class="material-bonus-color material-icons large-material-icons md:text-6xl text-4xl">{"play_arrow"}</span>
                                </button>
                                <ContextButton episode={episode.clone()} page_type={page_type.to_string()} />
                            }
                        </div>
                    }
                }




            </div>
        </div>
    }
}

pub fn download_episode_item(
    episode: Box<dyn EpisodeTrait>,
    description: String,
    is_expanded: bool,
    format_release: &str,
    on_play_click: Callback<MouseEvent>,
    on_shownotes_click: Callback<MouseEvent>,
    toggle_expanded: Callback<MouseEvent>,
    episode_duration: i32,
    listen_duration: Option<i32>,
    page_type: &str,
    on_checkbox_change: Callback<i32>,
    is_delete_mode: bool, // Add this line
    ep_url: String,
    completed: bool,
) -> Html {
    let span_duration = listen_duration.clone();
    let span_episode = episode_duration.clone();
    let formatted_duration = format_time(span_episode as f64);
    let formatted_listen_duration = span_duration.map(|ld| format_time(ld as f64));
    let listen_duration_percentage = listen_duration.map_or(0.0, |ld| {
        if episode_duration > 0 {
            (ld as f64 / episode_duration as f64) * 100.0
        } else {
            0.0
        }
    });
    let checkbox_ep = episode.get_episode_id();
    let should_show_buttons = !ep_url.is_empty();

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggleDescription(guid: &str, expanded: bool);
    }
    let description_class = if is_expanded {
        "desc-expanded".to_string()
    } else {
        "desc-collapsed".to_string()
    };

    html! {
        <div>
            <div class="item-container border-solid border flex items-start mb-4 shadow-md rounded-lg h-full">
                {if is_delete_mode {
                    html! {
                        <input type="checkbox" class="form-checkbox h-5 w-5 text-blue-600"
                            onchange={on_checkbox_change.reform(move |_| checkbox_ep)} />
                    }
                } else {
                    html! {}
                }}
                <div class="flex flex-col w-auto object-cover pl-4">
                    <img
                        src={episode.get_episode_artwork()}
                        alt={format!("Cover for {}", episode.get_episode_title())}
                        class="episode-image"
                    />
                </div>
                <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                    <div class="flex items-center space-x-2 cursor-pointer" onclick={on_shownotes_click}>
                        <p class="item_container-text episode-title font-semibold">
                            { episode.get_episode_title() }
                        </p>
                        {
                            if completed.clone() {
                                html! {
                                    <span class="material-bonus-color item_container-text material-icons text-md text-green-500">{"check_circle"}</span>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                    <hr class="my-2 border-t hidden md:block"/>
                    {
                        html! {
                            <div class="item-description-text hidden md:block">
                                <div class={format!("item_container-text episode-description-container {}", description_class)}>
                                    <SafeHtml html={description} />
                                </div>
                                <a class="link hover:underline cursor-pointer mt-4" onclick={toggle_expanded}>
                                    { if is_expanded { "See Less" } else { "See More" } }
                                </a>
                            </div>
                        }
                    }
                    <span class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2" style="flex-grow: 0; flex-shrink: 0; width: auto;">
                        <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                            <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                        </svg>
                        { format_release }
                    </span>
                    {
                        if completed {
                            html! {
                                <div class="flex items-center space-x-2">
                                    <span class="item_container-text">{ formatted_duration }</span>
                                    <span class="item_container-text">{ "-  Completed" }</span>
                                </div>
                            }
                        } else {
                            if formatted_listen_duration.is_some() {
                                html! {
                                    <div class="flex items-center space-x-2">
                                        <span class="item_container-text">{ formatted_listen_duration.clone() }</span>
                                        <div class="progress-bar-container">
                                            <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                        </div>
                                        <span class="item_container-text">{ formatted_duration }</span>
                                    </div>
                                }
                            } else {
                                html! {
                                    <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                }
                            }
                        }
                    }
                </div>
                {
                    html! {
                        <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;">
                            if should_show_buttons {
                                <button
                                    class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                    onclick={on_play_click}
                                >
                                    <span class="material-bonus-color material-icons large-material-icons md:text-6xl text-4xl">{"play_arrow"}</span>
                                </button>
                                <div class="show-on-large">
                                    <ContextButton episode={episode.clone()} page_type={page_type.to_string()} />
                                </div>
                            }
                        </div>
                    }
                }
            </div>
        </div>
    }
}

pub fn queue_episode_item(
    episode: Box<dyn EpisodeTrait>,
    description: String,
    is_expanded: bool,
    format_release: &str,
    on_play_click: Callback<MouseEvent>,
    on_shownotes_click: Callback<MouseEvent>,
    toggle_expanded: Callback<MouseEvent>,
    episode_duration: i32,
    listen_duration: Option<i32>,
    page_type: &str,
    on_checkbox_change: Callback<i32>,
    is_delete_mode: bool, // Add this line
    ep_url: String,
    completed: bool,
    ondragstart: Callback<DragEvent>,
    ondragenter: Callback<DragEvent>,
    ondragover: Callback<DragEvent>,
    ondrop: Callback<DragEvent>,
) -> Html {
    let span_duration = listen_duration.clone();
    let span_episode = episode_duration.clone();
    let formatted_duration = format_time(span_episode as f64);
    let formatted_listen_duration = span_duration.map(|ld| format_time(ld as f64));
    // Calculate the percentage of the episode that has been listened to
    let listen_duration_percentage = listen_duration.map_or(0.0, |ld| {
        if episode_duration > 0 {
            (ld as f64 / episode_duration as f64) * 100.0
        } else {
            0.0 // Avoid division by zero
        }
    });
    let checkbox_ep = episode.get_episode_id();
    let should_show_buttons = !ep_url.is_empty();

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window)]
        fn toggleDescription(guid: &str, expanded: bool);
    }
    let description_class = if is_expanded {
        "desc-expanded".to_string()
    } else {
        "desc-collapsed".to_string()
    };

    // // Wrap ontouchend callback to ensure class is removed
    // let wrapped_ontouchend = {
    //     let element_ref = element_ref_drag.clone();
    //     Callback::from(move |e: TouchEvent| {
    //         if let Some(element) = element_ref.cast::<HtmlElement>() {
    //             element.class_list().remove_1("dragging").unwrap();
    //         }
    //         ontouchend.emit(e);
    //     })
    // };

    html! {
        <>
        <div
            class="item-container border-solid border flex items-start mb-4 shadow-md rounded-lg h-full"
            draggable="true"
            ondragstart={ondragstart.clone()}
            ondragenter={ondragenter.clone()}
            ondragover={ondragover.clone()}
            ondrop={ondrop.clone()}
            data-id={episode.get_episode_id().to_string()}
        >
            <div class="drag-handle-wrapper flex items-center h-full">
                <button class="drag-handle" style="cursor: grab;">
                    <span class="material-icons">{"drag_indicator"}</span>
                </button>
            </div>
            {if is_delete_mode {
                    html! {
                        <input type="checkbox" class="form-checkbox h-5 w-5 text-blue-600"
                            onchange={on_checkbox_change.reform(move |_| checkbox_ep)} /> // Modify this line
                    }
                } else {
                    html! {}
                }}
                <div class="flex flex-col w-auto object-cover pl-4">
                    <img
                        src={episode.get_episode_artwork()}
                        alt={format!("Cover for {}", episode.get_episode_title())}
                        class="episode-image"
                    />
                </div>
                <div class="flex flex-col p-4 space-y-2 flex-grow md:w-7/12">
                    <div class="flex items-center space-x-2 cursor-pointer" onclick={on_shownotes_click}>
                        <p class="item_container-text episode-title font-semibold">
                            { episode.get_episode_title() }
                        </p>
                        {
                            if completed.clone() {
                                html! {
                                    <span class="material-bonus-color item_container-text material-icons text-md text-green-500">{"check_circle"}</span>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </div>
                    <hr class="my-2 border-t hidden md:block"/>
                    {
                        html! {
                            <div class="item-description-text hidden md:block">
                                <div class={format!("item_container-text episode-description-container {}", description_class)}>
                                    <SafeHtml html={description} />
                                </div>
                                <a class="link hover:underline cursor-pointer mt-4" onclick={toggle_expanded}>
                                    { if is_expanded { "See Less" } else { "See More" } }
                                </a>
                            </div>
                        }
                    }
                    <span class="episode-time-badge inline-flex items-center px-2.5 py-0.5 rounded me-2" style="flex-grow: 0; flex-shrink: 0; width: auto;">
                        <svg class="time-icon w-2.5 h-2.5 me-1.5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 20 20">
                            <path d="M10 0a10 10 0 1 0 10 10A10.011 10.011 0 0 0 10 0Zm3.982 13.982a1 1 0 0 1-1.414 0l-3.274-3.274A1.012 1.012 0 0 1 9 10V6a1 1 0 0 1 2 0v3.586l2.982 2.982a1 1 0 0 1 0 1.414Z"/>
                        </svg>
                        { format_release }
                    </span>
                    {
                        if completed {
                            html! {
                                <div class="flex items-center space-x-2">
                                    <span class="item_container-text">{ formatted_duration }</span>
                                    <span class="item_container-text">{ "-  Completed" }</span>
                                </div>
                            }
                        } else {
                            if formatted_listen_duration.is_some() {
                                html! {
                                    <div class="flex items-center space-x-2">
                                        <span class="item_container-text">{ formatted_listen_duration.clone() }</span>
                                        <div class="progress-bar-container">
                                            <div class="progress-bar" style={ format!("width: {}%;", listen_duration_percentage) }></div>
                                        </div>
                                        <span class="item_container-text">{ formatted_duration }</span>
                                    </div>
                                }
                            } else {
                                html! {
                                    <span class="item_container-text">{ format!("{}", formatted_duration) }</span>
                                }
                            }
                        }
                    }
                </div>
                {
                    html! {
                        <div class="flex flex-col items-center h-full w-2/12 px-2 space-y-4 md:space-y-8 button-container" style="align-self: center;">
                            if should_show_buttons {
                                <button
                                    class="item-container-button border-solid border selector-button font-bold py-2 px-4 rounded-full flex items-center justify-center md:w-16 md:h-16 w-10 h-10"
                                    onclick={on_play_click}
                                >
                                    <span class="material-bonus-color material-icons large-material-icons md:text-6xl text-4xl">{"play_arrow"}</span>
                                </button>
                                <ContextButton episode={episode.clone()} page_type={page_type.to_string()} />
                            }
                        </div>
                    }
                }




            </div>
            </>
    }
}
