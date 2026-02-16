
export const month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep","Oct", "Nov", "Dec"];

export  function fmt_date(ds: string){
      let dt= new Date(ds);
      return `${month_names[dt.getMonth()]} ${dt.getDate()}`;
  }

export  function fmt_hrs(hr: number){
      if (hr <= 12) { return `${hr}` }
      let nh = hr-12; //we wants 12hr clock
      if (nh < 10) { return `0${nh}`; }
      return `${nh}`;
  }
export  const fmt_min = (min: number) =>  (min < 10) ? `0${min}` : `${min}`;
export const fmt_secs = (sec: number) => (sec < 10) ? `0${sec}` : `${sec}`;

export function fmt_time(dts: string) {
        if (!dts) {
            console.log("error in fmt_time: dts is undefined");
            return fmt_time(new Date().toISOString())
        }
        let dt = new Date(dts);
        let hrs = dt.getHours();
        let ampm =  (hrs < 12) ? "AM" : "PM";
        return `${fmt_hrs(hrs)}:${fmt_min(dt.getMinutes())}:${fmt_secs(dt.getSeconds())} ${ampm}`;

}
