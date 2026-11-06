// Applies the colour scheme chosen with the theme button (theme_click.js) before the page is
// drawn, so a dark choice doesn't flash light first.
try {
	const theme = localStorage.getItem("nom-theme");
	if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme;
} catch {}
