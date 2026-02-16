import { onDestroy } from 'svelte';
import {writable} from "svelte/store";
import type { FrIdentity } from "./types";

//you'll notice liberal uses of the any type. why? because I don't care. I'm only using types for the intellisense
//not the type safety.

//from a float 0.9834234 to a string 98.34%
export function format_confidence (conf: number)  {
  const truncated = parseFloat(conf.toString().slice(0, (conf.toString().indexOf(".")) + 5)) * 100;
  return (conf >= 1) ? "100%" : `${truncated.toFixed(2)}%`;
}

//when you're tired of thinking about what the bottom value is for a particular type
export function null_undef (v: any) { return v === null || v === undefined }

/**
 * @ts-ignore
 */
export function onInterval(callback: () => void, milliseconds: number) {
  const interval = setInterval(callback, milliseconds);

  onDestroy(() => {
    clearInterval(interval);
  });
}

export function draw_bounding_box (ctx: any, min_confidence: number,  matches: FrIdentity []) {
 //if (null_undef(matches)) return;

  matches.forEach(function (match) {
    const face = match.face
    const box = face.bbox;
    if (box === undefined) return;
    const rectangle = new Path2D();

    rectangle.rect(box.origin.x, box.origin.y, box.width, box.height);
    //set color based on most likely status. again.. indexing to core_enrollments is a bit sketch.
    const conf = match.possible_matches[0].confidence;
    const details = match.possible_matches[0].details;
    //if no details were given, we can't know who this is
    if (null_undef(details)){
      ctx.strokeStyle = "gray";
    } else{
      const is_watch = match.possible_matches[0].details.status.includes("FR");
      ctx.strokeStyle = is_watch ? "red" : "green";
    }

    if (conf <= min_confidence) {
      ctx.strokeStyle = "gray";
    }
    ctx.lineWidth = 8;
    ctx.stroke(rectangle);
  });
}

//take an api call that fetches and wrap it in a cool function that uses stores that can update
//the user on things like.. in flight requests and errors. since we've got stores we can
//use these values reactively.
export function safe_call(fn: any) {

  const in_flight = writable(false);
  const error = writable({});
  const data = writable({});

  async function execute(params = []) {
    in_flight.set(true);
    error.set({});
    let res;

    try {
      if (params.length > 0) {
        // eslint-disable-next-line prefer-spread
        res = await fn(...params);//fn.apply(null, params);
      }
      else {
        res = await fn.apply();
      }

      if (res.code !== undefined && res.code !== 200) {
        console.log("we ain't found shit");
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        console.log(res);
        error.set(res);
        console.log("is we here");

      } else {
        data.set(res);

      }

    } catch(e) {
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      //-1 means custom code.
      const err = {code: -1, message: e.message};
      error.set(err);
      console.log(err);
    } finally {
      in_flight.set(false);
    }
  }

  return {
    in_flight,
    error,
    data,
    execute
  }
}

//wraps the console.log so that we can just turn it off instead of commenting out tons of log statements.
//must we still call the function and to an if check.. yeah. I don't see this as a problem, it's not like there's
//a log statement on every line.
export function logg(msg: string,obj: any=undefined) {
  const enable_log = true;
  if (enable_log) {
    if (null_undef(obj))
      console.log(msg);
    else
      console.log(msg,obj);
  }
}

//just a slightly more convenient way to check.. imho
export function is_array(val: any) {
  return Array.isArray(val);
}
