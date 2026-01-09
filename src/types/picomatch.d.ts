declare module "picomatch" {
  interface PicomatchOptions {
    dot?: boolean;
    bash?: boolean;
    nobrace?: boolean;
    noglobstar?: boolean;
    noextglob?: boolean;
    nonegate?: boolean;
    nocase?: boolean;
  }

  type Matcher = (input: string) => boolean;

  function picomatch(
    pattern: string | readonly string[],
    options?: PicomatchOptions,
  ): Matcher;

  export default picomatch;
}
