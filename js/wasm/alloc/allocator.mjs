const _ptr = Symbol("Boxed::pointer");
const _size = Symbol("Boxed::size");

// Obtain a fat pointer from a `Boxed` instance
const toFatPtr = (ptr, size) => {
  return (BigInt(ptr) << 32n) | BigInt(size);
};

// Parse a fat pointer, returning an array of two elements, in order the pointer
// to the memory and the size allocated.
const parseFatPtr = (value) => {
  let ptr = Number(value >> 32n);
  let size = Number(value & 0xffff_ffffn);

  return [ptr, size];
};

// The global object containing the allocation objects used to interact with
// the WebAssembly memory
function ALLOC({ memory, malloc, free, registry }) {
  Object.defineProperties(ALLOC, {
    memory: {
      value: memory,
    },
    malloc: {
      value: malloc,
    },
    free: {
      value: free,
    },
    registry: {
      value: registry,
    },
  });
}

// This function initialize the allocator based on a WebAssembly's Module
// instance.
export function init(instance) {
  const { malloc, free } = instance.exports;

  if (typeof malloc !== "function") {
    throw new TypeError("malloc must be a function");
  }

  if (typeof free !== "function") {
    throw new TypeError("free must be a function");
  }

  let memory = instance.exports.memory || (imports.env && imports.env.memory);
  const registry = new FinalizationRegistry((heldValue) => {
    let [ptr, size] = parseFatPtr(heldValue);

    free(ptr, size);
    console.log(`${size} bytes freed at location ${ptr}`);
  });

  ALLOC({
    memory,
    registry,
    malloc,
    free,
  });

  return { malloc, free, memory };
}

// The `Boxed` class is used to creates types that can be allocated or are
// already allocated on the heap of the WebAssembly memory.
// If an allocation is requested, it will beautomatically garbaged collected.
export class Boxed {
  static #typeSize;

  static get size() {
    this.#typeSize;
  }

  static from(ptr) {
    return new Boxed({ ptr, size: this.size }).downcast(this);
  }

  downcast(type) {
    if (type.size !== this.size) {
      throw new TypeError("Cannot downcast between objects of different size");
    }

    let instance = Object.create(type.prototype);

    instance[_ptr] = this[_ptr];
    instance[_size] = this[_size];

    return instance;
  }

  constructor({ ptr, size, value }) {
    if (typeof ALLOC.memory === "undefined") {
      throw new ReferenceError("ENV.memory is not initialized");
    }

    if (typeof ALLOC.registry === "undefined") {
      throw new ReferenceError("ENV.registry is not initialized");
    }

    if (typeof ALLOC.malloc !== "function") {
      throw new TypeError("ENV.malloc must be a function");
    }

    if (typeof size !== "number") {
      throw new TypeError("Size must be a number");
    }

    // If a pointer or fat pointer is given, then it means it was already
    // allocated
    if (typeof ptr === "number") {
      this[_ptr] = ptr;
    } else if (typeof ptr === "bigint") {
      let [ptr, fsize] = parseFatPtr(ptr);

      if (fsize !== size) {
        throw new ReferenceError(
          "Fat Pointer's size should match the given size"
        );
      }

      this[_ptr] = ptr;
    } else {
      // No valid pointer given, so an allocation is required
      this[_ptr] = ALLOC.malloc(size);
      // Since we're allocated from JavaScript side, we also want to deallocate
      // once the object is garbage collected
      ALLOC.registry.register(this, toFatPtr(this.ptr, size), this);
    }

    this[_size] = size;

    // If a value was given we also want to write that in memory
    if (typeof value !== "undefined" && value !== null) {
      this.write(ALLOC.memory, value);
    }
  }

  get ptr() {
    return this[_ptr];
  }

  get size() {
    return this[_size];
  }

  get value() {
    return this.read(ALLOC.memory);
  }

  set value(value) {
    this.write(ALLOC.memory, value);
  }

  [Symbol.toPrimitive](hint) {
    if (hint === "number" || hint === "default") {
      return this.ptr;
    }
  }

  forget() {
    ALLOC.registry.unregister(this);
  }

  drop() {
    this.forget();
    ALLOC.free(this.ptr, this.size);
  }

  read(memory) {}

  write(memory, value) {}
}
