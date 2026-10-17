// Copy buttons (<button data-copy="element-id">), the theme switch (<button data-theme-toggle>)
// filter bars (<form data-autosubmit>) that apply a select as soon as it changes, and dialogs:
// <button data-dialog="id"> opens <dialog id>, [data-close] closes it, and <dialog data-open> opens
// on load because its form came back with an error. A submit button with data-confirm asks first
// in the shared #confirm dialog; without this script the form submits at once. An icon picker
// (<input type="file" data-icon-file>) scales the picture to a small square before it is sent.
// theme.js applies a saved theme before the first paint.
document.addEventListener("click", (event) => {
  if (!event.target.closest("[data-theme-toggle]")) return;
  const root = document.documentElement;
  const dark = root.dataset.theme
    ? root.dataset.theme === "dark"
    : window.matchMedia("(prefers-color-scheme: dark)").matches;
  root.dataset.theme = dark ? "light" : "dark";
  try {
    localStorage.setItem("monita-theme", root.dataset.theme);
  } catch {}
});

document.addEventListener("click", async (event) => {
  const button = event.target.closest("[data-copy]");
  if (!button) return;
  const source = document.getElementById(button.dataset.copy);
  if (!source) return;
  const label = (button.dataset.label ??= button.textContent);
  setTimeout(() => (button.textContent = label), 2000);
  try {
    await navigator.clipboard.writeText(source.textContent.trim());
    button.textContent = "Copied";
  } catch {
    const range = document.createRange();
    range.selectNodeContents(source);
    const selection = window.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    button.textContent = "Press Ctrl+C";
  }
});

document.addEventListener("change", (event) => {
  const form = event.target.closest("form[data-autosubmit]");
  if (form && event.target.matches("select")) form.requestSubmit();
});

// showModal() would focus the close button first. Focus the first field instead, or Cancel when
// there is none, so Enter never confirms a risky action by default.
function openDialog(dialog) {
  if (!dialog) return;
  dialog.showModal();
  const field = dialog.querySelector(".dialog__body :is(input, select, textarea)");
  (field ?? dialog.querySelector(".dialog__foot [data-close]"))?.focus();
}

document.addEventListener("click", (event) => {
  const opener = event.target.closest("[data-dialog]");
  if (opener) openDialog(document.getElementById(opener.dataset.dialog));
  if (event.target.closest("[data-close]")) event.target.closest("dialog")?.close();
  // A click on the backdrop lands on the dialog element itself.
  if (event.target.matches("dialog")) event.target.close();
});
for (const dialog of document.querySelectorAll("dialog[data-open]")) openDialog(dialog);

document.addEventListener("submit", (event) => {
  const form = event.target;
  const button = event.submitter;
  if (!button?.dataset.confirm || form.dataset.confirmed) return;
  event.preventDefault();
  const dialog = document.getElementById("confirm");
  dialog.querySelector("#confirm-title").textContent = button.dataset.confirm;
  dialog.querySelector("#confirm-text").textContent = button.dataset.confirmText ?? "";
  const yes = dialog.querySelector("#confirm-yes");
  yes.textContent = button.textContent.trim();
  yes.onclick = () => {
    form.dataset.confirmed = "yes";
    dialog.close();
    form.requestSubmit(button);
  };
  openDialog(dialog);
});

// The server stores at most 32 KB and never resizes, so the page sends a 128 px square: WebP,
// or PNG where the browser cannot write WebP, at the first quality that fits.
const ICON_PX = 128;
const ICON_MAX = 32 * 1024;
document.addEventListener("change", async (event) => {
  const input = event.target.closest("[data-icon-file]");
  if (!input) return;
  const field = input.closest(".field");
  const value = field.querySelector("[data-icon-value]");
  const preview = field.querySelector("[data-icon-preview]");
  value.value = "";
  preview.hidden = true;
  input.setCustomValidity("");
  const file = input.files[0];
  if (!file) return;
  try {
    const picture = await createImageBitmap(file);
    const side = Math.min(picture.width, picture.height);
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = ICON_PX;
    const x = (picture.width - side) / 2;
    const y = (picture.height - side) / 2;
    canvas.getContext("2d").drawImage(picture, x, y, side, side, 0, 0, ICON_PX, ICON_PX);
    const url = [0.85, 0.6, 0.4]
      .map((quality) => canvas.toDataURL("image/webp", quality))
      .find((u) => u.length * 0.75 <= ICON_MAX);
    if (!url) throw new Error("icon over the cap");
    value.value = url;
    preview.src = url;
    preview.hidden = false;
  } catch {
    input.setCustomValidity("Monita could not read this picture. Pick a PNG, JPEG or WebP file.");
    input.reportValidity();
  }
});
