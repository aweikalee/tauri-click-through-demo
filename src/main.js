const { invoke } = window.__TAURI__.tauri

const clickThrough = document.getElementById("click-through")
clickThrough.addEventListener("mouseenter", () => {
  invoke("set_ignore_cursor_events", {
    ignore: true,
    forward: true,
  })
})
clickThrough.addEventListener("mouseleave", () => {
  invoke("set_ignore_cursor_events", {
    ignore: false,
    forward: true,
  })
})
