// Shows how far the download of STAMP has got, on the page's loading
// screen. trunk (https://trunkrs.dev) calls these functions while it fetches
// and starts the WebAssembly program; see index.html.
export default function initializer() {
  const element = (id) => document.getElementById(id);
  const megabytes = (bytes) => (bytes / 1048576).toFixed(1) + " MB";
  // A bar that fills, when the size of the download is known, or one that
  // keeps moving, when it is not (servers that compress the program report
  // the compressed size, which the bytes received soon pass).
  const show = (fraction) => {
    const track = element("stamp_track");
    const bar = element("stamp_bar");
    if (!track || !bar) return;
    if (fraction === null) {
      track.classList.add("busy");
      bar.style.width = "";
    } else {
      track.classList.remove("busy");
      bar.style.width = (100 * fraction).toFixed(1) + "%";
    }
  };
  const say = (id, text) => {
    const target = element(id);
    if (target) target.textContent = text;
  };
  return {
    onStart: () => show(null),
    onProgress: ({ current, total }) => {
      if (total > 0 && current <= total) {
        show(current / total);
        say("stamp_detail", `${megabytes(current)} of ${megabytes(total)}`);
      } else {
        show(null);
        say("stamp_detail", `${megabytes(current)} received`);
      }
    },
    onComplete: () => {
      say("stamp_message", "Starting STAMP...");
      say("stamp_detail", "");
      show(null);
    },
    onSuccess: () => {},
    onFailure: (error) => {
      say("stamp_message", "STAMP could not be loaded.");
      say("stamp_detail", String(error));
      const track = element("stamp_track");
      if (track) track.style.display = "none";
    },
  };
}
