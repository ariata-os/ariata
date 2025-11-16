<script lang="ts">
	import { marked } from "marked";
	import DOMPurify from "isomorphic-dompurify";

	interface Props {
		content: string;
	}

	let { content }: Props = $props();

	// Configure marked options
	marked.setOptions({
		breaks: false, // Don't convert single \n to <br> (requires double space or double newline)
		gfm: true, // GitHub Flavored Markdown
	});

	// Parse and sanitize markdown
	let html = $derived.by(() => {
		const rawHtml = marked.parse(content) as string;
		return DOMPurify.sanitize(rawHtml, {
			ALLOWED_TAGS: [
				"h1",
				"h2",
				"h3",
				"h4",
				"h5",
				"h6",
				"p",
				"br",
				"strong",
				"em",
				"del",
				"s",
				"code",
				"pre",
				"ul",
				"ol",
				"li",
				"blockquote",
				"a",
				"hr",
				"table",
				"thead",
				"tbody",
				"tr",
				"th",
				"td",
				"img",
			],
			ALLOWED_ATTR: ["href", "title", "src", "alt", "class"],
		});
	});
</script>

<div class="markdown">
	{@html html}
</div>
