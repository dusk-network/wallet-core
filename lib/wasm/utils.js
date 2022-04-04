/* global BigInt */

const bigThirtyTwo = BigInt(32);
const bigSixteen = BigInt(16);

export function getUint64BigInt(dataview, byteOffset, littleEndian = true) {
  const left = BigInt(dataview.getUint32(byteOffset | 0, !!littleEndian) >>> 0);
  const right = BigInt(dataview.getUint32(((byteOffset | 0) + 4) | 0, !!littleEndian) >>> 0);
  return littleEndian ? (right << bigThirtyTwo) | left : (left << bigThirtyTwo) | right;
}

export function getUint32BigInt(dataview, byteOffset, littleEndian = true) {
  const left = BigInt(dataview.getUint16(byteOffset | 0, !!littleEndian) >>> 0);
  const right = BigInt(dataview.getUint16(((byteOffset | 0) + 4) | 0, !!littleEndian) >>> 0);
  return littleEndian ? (right << bigSixteen) | left : (left << bigSixteen) | right;
}
