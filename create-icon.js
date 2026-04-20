// Simple ICO/PNG generator without dependencies
// Run with: node create-icon.js

const fs = require('fs');
const path = require('path');

const iconDir = path.join(__dirname, 'icons');
if (!fs.existsSync(iconDir)) {
    fs.mkdirSync(iconDir, { recursive: true });
}

// Simple 32x32 PNG with blue background and lightning
// This is a minimal valid PNG file
const pngData = Buffer.from([
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
    0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
    0x49, 0x48, 0x44, 0x52, // IHDR
    0x00, 0x00, 0x00, 0x20, // width: 32
    0x00, 0x00, 0x00, 0x20, // height: 32
    0x08, 0x06, // bit depth: 8, color type: RGBA
    0x00, 0x00, 0x00, // compression, filter, interlace
    0x73, 0x7A, 0x7A, 0xF4, // CRC
]);

// Generate a simple colored PNG using raw RGBA data
function createSimplePng(size) {
    const { createCanvas } = require('canvas');
    const canvas = createCanvas(size, size);
    const ctx = canvas.getContext('2d');
    
    // Gradient background
    const gradient = ctx.createLinearGradient(0, 0, size, size);
    gradient.addColorStop(0, '#2563eb');
    gradient.addColorStop(1, '#764ba2');
    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.roundRect(0, 0, size, size, size / 6);
    ctx.fill();
    
    // Lightning bolt
    ctx.fillStyle = 'white';
    ctx.font = `bold ${size * 0.6}px sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('⚡', size / 2, size / 2);
    
    return canvas.toBuffer('image/png');
}

// Create a minimal valid ICO with embedded PNG
function createMinimalIco(pngBuffer) {
    // ICO header: 6 bytes
    const header = Buffer.alloc(6);
    header.writeUInt16LE(0, 0);      // Reserved
    header.writeUInt16LE(1, 2);      // Type: 1 = ICO
    header.writeUInt16LE(1, 4);      // Number of images
    
    // ICO directory entry: 16 bytes
    const entry = Buffer.alloc(16);
    entry.writeUInt8(0, 0);          // Width (0 = 256)
    entry.writeUInt8(0, 1);          // Height (0 = 256)
    entry.writeUInt8(0, 2);          // Color palette
    entry.writeUInt8(0, 3);          // Reserved
    entry.writeUInt16LE(1, 4);       // Color planes
    entry.writeUInt16LE(32, 6);      // Bits per pixel
    entry.writeUInt32LE(pngBuffer.length, 8);   // Size
    entry.writeUInt32LE(22, 12);     // Offset (6 + 16 = 22)
    
    return Buffer.concat([header, entry, pngBuffer]);
}

// Since we don't have canvas, create a super minimal placeholder
// This creates a valid 32x32 blue square PNG
function createMinimalBluePng() {
    const width = 32;
    const height = 32;
    
    // PNG signature
    const signature = Buffer.from([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    
    // IHDR chunk
    const ihdrData = Buffer.alloc(13);
    ihdrData.writeUInt32BE(width, 0);   // Width
    ihdrData.writeUInt32BE(height, 4); // Height
    ihdrData.writeUInt8(8, 8);          // Bit depth
    ihdrData.writeUInt8(6, 9);          // Color type (RGBA)
    ihdrData.writeUInt8(0, 10);         // Compression
    ihdrData.writeUInt8(0, 11);         // Filter
    ihdrData.writeUInt8(0, 12);         // Interlace
    
    const ihdrChunk = createChunk('IHDR', ihdrData);
    
    // Create raw image data (RGBA)
    const rawData = [];
    for (let y = 0; y < height; y++) {
        rawData.push(0); // Filter byte
        for (let x = 0; x < width; x++) {
            // Blue-purple gradient
            const r = 37;
            const g = 99;
            const b = 235;
            rawData.push(r, g, b, 255);
        }
    }
    
    // Compress with zlib (simple deflate)
    const zlib = require('zlib');
    const compressed = zlib.deflateSync(Buffer.from(rawData));
    const idatChunk = createChunk('IDAT', compressed);
    
    // IEND chunk
    const iendChunk = createChunk('IEND', Buffer.alloc(0));
    
    return Buffer.concat([signature, ihdrChunk, idatChunk, iendChunk]);
}

function createChunk(type, data) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length, 0);
    
    const typeBuffer = Buffer.from(type);
    const crcData = Buffer.concat([typeBuffer, data]);
    
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(crcData), 0);
    
    return Buffer.concat([length, typeBuffer, data, crc]);
}

// CRC32 implementation
function crc32(data) {
    let crc = 0xFFFFFFFF;
    const table = makeCrcTable();
    for (let i = 0; i < data.length; i++) {
        crc = (crc >>> 8) ^ table[(crc ^ data[i]) & 0xFF];
    }
    return (crc ^ 0xFFFFFFFF) >>> 0;
}

function makeCrcTable() {
    const table = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
        let c = n;
        for (let k = 0; k < 8; k++) {
            c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
        }
        table[n] = c;
    }
    return table;
}

// Generate icons
try {
    const pngBuffer = createMinimalBluePng();
    
    fs.writeFileSync(path.join(iconDir, 'icon.png'), pngBuffer);
    console.log('PNG created: icons/icon.png');
    
    const icoBuffer = createMinimalIco(pngBuffer);
    fs.writeFileSync(path.join(iconDir, 'icon.ico'), icoBuffer);
    console.log('ICO created: icons/icon.ico');
    
    console.log('Done!');
} catch (e) {
    console.error('Error:', e.message);
}
