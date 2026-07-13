<script lang="ts">
  type DocumentBlock =
    | { kind: "title" | "heading" | "paragraph"; text: string }
    | { kind: "list"; items: string[] };

  let { text }: { text: string } = $props();
  const blocks = $derived(parseDocument(text));

  function parseDocument(source: string): DocumentBlock[] {
    const result: DocumentBlock[] = [];
    let paragraph: string[] = [];
    let list: string[] = [];

    function flushParagraph(): void {
      if (!paragraph.length) return;
      result.push({ kind: "paragraph", text: clean(paragraph.join(" ")) });
      paragraph = [];
    }

    function flushList(): void {
      if (!list.length) return;
      result.push({ kind: "list", items: list.map(clean) });
      list = [];
    }

    for (const rawLine of source.split("\n")) {
      const line = rawLine.trim();
      if (!line) {
        flushParagraph();
        flushList();
      } else if (line.startsWith("# ")) {
        flushParagraph();
        flushList();
        result.push({ kind: "title", text: clean(line.slice(2)) });
      } else if (line.startsWith("## ")) {
        flushParagraph();
        flushList();
        result.push({ kind: "heading", text: clean(line.slice(3)) });
      } else if (line.startsWith("- ")) {
        flushParagraph();
        list.push(line.slice(2));
      } else {
        flushList();
        paragraph.push(line);
      }
    }
    flushParagraph();
    flushList();
    return result;
  }

  function clean(value: string): string {
    return value.replaceAll("**", "").replaceAll("`", "");
  }
</script>

<article class="legal-document">
  {#each blocks as block}
    {#if block.kind === "title"}
      <h3>{block.text}</h3>
    {:else if block.kind === "heading"}
      <h4>{block.text}</h4>
    {:else if block.kind === "list"}
      <ul>
        {#each block.items as item}<li>{item}</li>{/each}
      </ul>
    {:else}
      <p>{block.text}</p>
    {/if}
  {/each}
</article>

<style>
  .legal-document {
    padding: 20px 22px 28px;
    color: var(--color-text-muted);
    font-size: 12px;
    line-height: 1.62;
  }
  h3,
  h4,
  p,
  ul {
    margin: 0;
  }
  h3 {
    color: var(--color-text);
    font-family: Georgia, serif;
    font-size: 22px;
    font-weight: 500;
  }
  h4 {
    margin-top: 22px;
    color: var(--color-highlight);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  p,
  ul {
    margin-top: 10px;
  }
  ul {
    display: grid;
    gap: 5px;
    padding-left: 20px;
  }
</style>
