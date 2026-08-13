use std::rc::Rc;
use yew::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGlRenderingContext as GL};
use shared_core::i18n::Language;
use shared_frontend::AppShell;
use shared_frontend::HeaderProps;

#[function_component(FallbackApp)]
fn fallback_app() -> Html {
    let connected = use_state(|| false);

    // Polling effect
    {
        let connected = connected.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let mut attempts = 0;
                loop {
                    if let Some(window) = web_sys::window() {
                        let mut opts = web_sys::RequestInit::new();
                        opts.set_method("GET");
                        if let Ok(request) = web_sys::Request::new_with_str_and_init("http://localhost:8080/health", &opts) {
                            if let Ok(resp_val) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
                                if let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() {
                                    if resp.status() == 200 {
                                        connected.set(true);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    attempts += 1;
                    if attempts > 100 { break; } // limit
                    
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        if let Some(window) = web_sys::window() {
                            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 2000);
                        }
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                }
            });
            || ()
        });
    }

    // Canvas effect
    let canvas_ref = use_node_ref();
    {
        let canvas_ref = canvas_ref.clone();
        let connected = connected.clone();
        use_effect_with(canvas_ref, move |canvas_ref| {
            if *connected { return Box::new(|| ()) as Box<dyn FnOnce()>; }
            let canvas = match canvas_ref.cast::<HtmlCanvasElement>() {
                Some(c) => c,
                None => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };
            
            let gl = match canvas.get_context("webgl") {
                Ok(Some(ctx)) => match ctx.dyn_into::<GL>() {
                    Ok(gl) => gl,
                    Err(_) => return Box::new(|| ()) as Box<dyn FnOnce()>,
                },
                _ => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };

            let vert_shader = match gl.create_shader(GL::VERTEX_SHADER) {
                Some(s) => s,
                None => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };
            gl.shader_source(&vert_shader, "attribute vec2 position; void main() { gl_Position = vec4(position, 0.0, 1.0); }");
            gl.compile_shader(&vert_shader);

            let frag_shader = match gl.create_shader(GL::FRAGMENT_SHADER) {
                Some(s) => s,
                None => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };
            gl.shader_source(&frag_shader, "
                precision mediump float;
                uniform float time;
                uniform vec2 resolution;
                void main() {
                    vec2 uv = gl_FragCoord.xy / resolution;
                    vec3 col = 0.5 + 0.5 * cos(time + uv.xyx + vec3(0,2,4));
                    uv += 0.1 * vec2(sin(time + uv.y * 10.0), cos(time + uv.x * 10.0));
                    float fluid = sin(uv.x * 20.0) * cos(uv.y * 20.0);
                    gl_FragColor = vec4(col + fluid * 0.2, 1.0);
                }
            ");
            gl.compile_shader(&frag_shader);

            let program = match gl.create_program() {
                Some(p) => p,
                None => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };
            gl.attach_shader(&program, &vert_shader);
            gl.attach_shader(&program, &frag_shader);
            gl.link_program(&program);
            gl.use_program(Some(&program));

            let vertices: [f32; 12] = [
                -1.0, -1.0, 1.0, -1.0, -1.0, 1.0,
                -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
            ];
            let buffer = match gl.create_buffer() {
                Some(b) => b,
                None => return Box::new(|| ()) as Box<dyn FnOnce()>,
            };
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
            unsafe {
                let vert_array = js_sys::Float32Array::view(&vertices);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &vert_array, GL::STATIC_DRAW);
            }

            let position = gl.get_attrib_location(&program, "position") as u32;
            gl.enable_vertex_attrib_array(position);
            gl.vertex_attrib_pointer_with_i32(position, 2, GL::FLOAT, false, 0, 0);

            let time_loc = gl.get_uniform_location(&program, "time");
            let res_loc = gl.get_uniform_location(&program, "resolution");

            let render_loop: Rc<std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(std::cell::RefCell::new(None));
            let render_loop_clone = render_loop.clone();

            let mut start_time = None;
            *render_loop.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
                if start_time.is_none() { start_time = Some(time); }
                let t = (time - start_time.unwrap_or(time)) * 0.001;

                let width = canvas.client_width() as f32;
                let height = canvas.client_height() as f32;
                canvas.set_width(canvas.client_width() as u32);
                canvas.set_height(canvas.client_height() as u32);
                gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
                
                gl.uniform1f(time_loc.as_ref(), t as f32);
                gl.uniform2f(res_loc.as_ref(), width, height);

                gl.draw_arrays(GL::TRIANGLES, 0, 6);

                if let Some(window) = web_sys::window() {
                    if let Some(ref cb) = *render_loop_clone.borrow() {
                        let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
                    }
                }
            }) as Box<dyn FnMut(f64)>));

            if let Some(window) = web_sys::window() {
                if let Some(ref cb) = *render_loop.borrow() {
                    let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }

            Box::new(move || {
                let _ = render_loop.borrow_mut().take();
            }) as Box<dyn FnOnce()>
        });
    }

    let header = yew::props!(HeaderProps {
        site_title: "Studio2201 Fallback".to_string(),
        language: Language::English,
        toggle_theme: Callback::from(|_| ()),
        on_language_change: Callback::from(|_| ()),
        is_authenticated: false,
        pin_required: false,
        on_logout: Callback::from(|_| ()),
        on_print: None,
        print_disabled: false,
    });

    html! {
        <AppShell header={header} use_container={false}>
            <div style="width: 100%; height: 100vh; position: relative; background: #000; overflow: hidden;">
                if *connected {
                    <div style="position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); color: white; font-family: monospace; font-size: 2rem;">
                        {"Upstream Connected!"}
                    </div>
                } else {
                    <canvas ref={canvas_ref} style="width: 100%; height: 100%; display: block;" />
                    <div style="position: absolute; top: 20px; left: 20px; color: rgba(255,255,255,0.7); font-family: monospace;">
                        {"Polling upstream server..."}
                    </div>
                }
            </div>
        </AppShell>
    }
}

pub fn run() {
    yew::Renderer::<FallbackApp>::new().render();
}
