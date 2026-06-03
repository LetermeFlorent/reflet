import sharp from "sharp";

const svg = `
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#3AA3FF"/>
      <stop offset="1" stop-color="#0A66FF"/>
    </linearGradient>
    <filter id="shadow" x="-30%" y="-30%" width="160%" height="160%">
      <feDropShadow dx="0" dy="16" stdDeviation="26" flood-color="#001a4d" flood-opacity="0.28"/>
    </filter>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="224" fill="url(#bg)"/>
  <rect x="64" y="64" width="896" height="896" rx="224" fill="#ffffff" opacity="0.06"/>
  <g filter="url(#shadow)">
    <circle cx="512" cy="512" r="236" fill="none" stroke="#ffffff" stroke-width="48"/>
    <path d="M512 300 A212 212 0 0 1 512 724 Z" fill="#ffffff"/>
  </g>
</svg>`;

await sharp(Buffer.from(svg)).resize(1024, 1024).png().toFile("app-icon.png");
console.log("app-icon.png (1024x1024) generated");
