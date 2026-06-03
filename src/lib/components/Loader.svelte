<div class="boot">
  <span class="brand">◑</span>
  <div class="bar"><span></span></div>
</div>

<style>
  .boot {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s4);
    animation: fadein 0.2s var(--ease);
  }
  .brand {
    font-size: 38px;
    line-height: 1;
    color: var(--accent);
    opacity: 0.55;
  }
  .bar {
    width: 200px;
    height: 6px;
    border-radius: var(--r-control);
    background: var(--hover);
    overflow: hidden;
    position: relative;
  }
  .bar span {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    width: 40%;
    border-radius: var(--r-control);
    background: var(--accent);
    animation: slide 1.1s var(--ease) infinite;
  }
  @keyframes slide {
    0% {
      transform: translateX(-120%);
    }
    100% {
      transform: translateX(330%);
    }
  }
  @keyframes fadein {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar span {
      animation: none;
      width: 100%;
      opacity: 0.5;
    }
  }
</style>
