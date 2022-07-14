import { Boxed } from "./allocator.mjs";

/**
 * Class that represents a Boxed Unsigned Integer of 8 bit
 */
class Uint8 extends Boxed {
  static get size() {
    return 1;
  }

  constructor(value) {
    super({ size: Uint8.size, value });
  }

  write(memory, value) {
    let view = new DataView(memory.buffer);

    view.setUint8(this.ptr, value, true);
  }

  read(memory) {
    let view = new DataView(memory.buffer);

    return view.getUint8(this.ptr, true);
  }
}

/**
 * Class that represents a Boxed Unsigned Integer of 16 bit
 */
class Uint16 extends Boxed {
  static get size() {
    return 2;
  }

  constructor(value) {
    super({ size: Uint16.size, value });
  }

  write(memory, value) {
    let view = new DataView(memory.buffer);

    view.setUint16(this.ptr, value, true);
  }

  read(memory) {
    let view = new DataView(memory.buffer);

    return view.getUint16(this.ptr, true);
  }
}

/**
 * Class that represents a Boxed Unsigned Integer of 32 bit
 */
class Uint32 extends Boxed {
  static get size() {
    return 4;
  }

  constructor(value) {
    super({ size: Uint32.size, value });
  }

  write(memory, value) {
    let view = new DataView(memory.buffer);

    view.setUint32(this.ptr, value, true);
  }

  read(memory) {
    let view = new DataView(memory.buffer);

    return view.getUint32(this.ptr, true);
  }
}

/**
 * Class that represents a Boxed Unsigned Integer of 64 bit
 */
class Uint64 extends Boxed {
  static get size() {
    return 8;
  }

  constructor(value) {
    if (typeof value === "number") {
      value = BigInt(value);
    }

    super({ size: Uint64.size, value });
  }

  write(memory, value) {
    if (typeof value === "number") {
      value = BigInt(value);
    }

    let view = new DataView(memory.buffer);

    view.setBigUint64(this.ptr, value, true);
  }

  read(memory) {
    let view = new DataView(memory.buffer);

    return view.getBigUint64(this.ptr, true);
  }
}

export class BoxedBuffer extends Boxed {
  constructor({ ptr, size, value }) {
    super({ ptr, size, value });

    if (
      typeof value !== "undefined" &&
      (!(value instanceof Uint8Array) || value.length !== size)
    ) {
      throw new TypeError(`Value must be a Uint8Array of size ${size}`);
    }
  }

  write(memory, value) {
    new Uint8Array(memory.buffer).set(value, this.ptr);
  }

  read(memory) {
    return new Uint8Array(memory.buffer, this.ptr, this.size);
  }
}

export class Buffer64 extends BoxedBuffer {
  static get size() {
    return 64;
  }

  constructor({ ptr, size, value }) {
    super({ ptr, size: Buffer64.size, value });
  }

  write(memory, value) {
    new Uint8Array(memory.buffer).set(value, this.ptr);
  }

  read(memory) {
    return new Uint8Array(memory.buffer, this.ptr, this.size);
  }
}

export const u8 = (n) => new Uint8(n);
export const u16 = (n) => new Uint16(n);
export const u32 = (n) => new Uint32(n);
export const u64 = (n) => new Uint64(n);

u8.from = Uint8.from.bind(Uint8);
u16.from = Uint16.from.bind(Uint16);
u32.from = Uint32.from.bind(Uint32);
u64.from = Uint64.from.bind(Uint64);
