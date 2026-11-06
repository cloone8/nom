// The theme button's click: cycles <html data-theme> through none (Auto, follows the system),
// "light" and "dark", and remembers the choice for theme_init.js.
const root = document.documentElement;
const theme = { "": "light", light: "dark" }[root.dataset.theme ?? ""];
if (theme) root.dataset.theme = theme;
else delete root.dataset.theme;
try {
	if (theme) localStorage.setItem("nom-theme", theme);
	else localStorage.removeItem("nom-theme");
} catch {}
