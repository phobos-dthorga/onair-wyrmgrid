<script lang="ts">
  import heroDark from "$brand/key-art/derivatives/hero-dark.png";
  import heroLight from "$brand/key-art/derivatives/hero-light.png";
  import { launchArtworkTone } from "./presentation";

  type Props = {
    eyebrow: string;
    message: string;
    canvas: string;
    error?: boolean;
    retryLabel?: string;
    onretry?: () => void;
  };

  let {
    eyebrow,
    message,
    canvas,
    error = false,
    retryLabel = "Try again",
    onretry,
  }: Props = $props();

  const artwork = $derived(
    launchArtworkTone(canvas) === "light" ? heroLight : heroDark,
  );
</script>

<main
  class:error
  class="launch-screen"
  aria-live={error ? "assertive" : "polite"}
>
  <img class="launch-artwork" src={artwork} alt="" aria-hidden="true" />
  <div class="launch-veil" aria-hidden="true"></div>
  <section class="launch-status">
    <span>{eyebrow}</span>
    <strong>{message}</strong>
    {#if error && onretry}
      <button type="button" onclick={onretry}>{retryLabel}</button>
    {/if}
  </section>
</main>

<style>
  .launch-screen {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    color: #e9f1ef;
    background: #07110f;
  }

  .launch-artwork,
  .launch-veil {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .launch-artwork {
    object-fit: cover;
    object-position: left center;
  }

  .launch-veil {
    background:
      linear-gradient(180deg, transparent 52%, rgba(2, 9, 8, 0.74) 100%),
      linear-gradient(90deg, rgba(2, 9, 8, 0.16), transparent 58%);
  }

  .launch-status {
    position: absolute;
    left: clamp(28px, 5vw, 76px);
    bottom: clamp(28px, 7vh, 64px);
    display: grid;
    gap: 9px;
    width: min(470px, calc(100vw - 56px));
    padding: 15px 18px 16px;
    border-left: 2px solid #73d6ad;
    background: rgba(4, 15, 13, 0.78);
    box-shadow: 0 18px 56px rgba(0, 0, 0, 0.34);
    backdrop-filter: blur(10px);
  }

  .launch-status span {
    color: #d5ae5f;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.17em;
    text-transform: uppercase;
  }

  .launch-status strong {
    color: #d8e4e0;
    font-size: 13px;
    font-weight: 500;
    line-height: 1.5;
  }

  .error .launch-status {
    border-left-color: #ed8074;
  }

  .error .launch-status span {
    color: #ed8074;
  }

  button {
    justify-self: start;
    margin-top: 3px;
    border: 1px solid rgba(115, 214, 173, 0.46);
    border-radius: 3px;
    padding: 8px 13px;
    color: #07110f;
    background: #73d6ad;
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }

  button:focus-visible {
    outline: 2px solid #e9f1ef;
    outline-offset: 3px;
  }
</style>
