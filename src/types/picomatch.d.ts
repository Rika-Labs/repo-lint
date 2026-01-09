declare module "picomatch" {
  interface PicomatchOptions {
    dot?: boolean;
    bash?: boolean;
    nobrace?: boolean;
    nocase?: boolean;
    nonegate?: boolean;
    noext?: boolean;
    noglobstar?: boolean;
  }

  type MatcherFn = (input: string) => boolean;

  function picomatch(pattern: string, options?: PicomatchOptions): MatcherFn;
  function picomatch(patterns: string[], options?: PicomatchOptions): MatcherFn;

  export default picomatch;
}
