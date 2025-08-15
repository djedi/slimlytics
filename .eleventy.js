module.exports = (eleventyConfig) => {
	eleventyConfig.addPassthroughCopy({ "src/public/images": "images" });
	eleventyConfig.addPassthroughCopy({ "src/public/css": "css" });
	eleventyConfig.addPassthroughCopy({ "src/public/js": "js" });
	eleventyConfig.addPassthroughCopy({
		"node_modules/htmx.org/dist/htmx.min.js": "js/htmx.min.js",
	});
	eleventyConfig.addPassthroughCopy({
		"node_modules/alpinejs/dist/cdn.min.js": "js/alpine.min.js",
	});

	eleventyConfig.addWatchTarget("src/dashboard/");

	return {
		dir: {
			input: "src/dashboard",
			output: "dist",
			includes: "_includes",
			data: "_data",
		},
		templateFormats: ["html", "njk", "md"],
		htmlTemplateEngine: "njk",
	};
};
