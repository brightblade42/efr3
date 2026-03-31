/** @type {import('tailwindcss').Config} */
const colors = require("tailwindcss/colors.js");
module.exports = {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {


      colors: {

        bgray: colors.gray,
        wgray: colors.stone,
        lime: colors.lime,
        teal: colors.teal

      }
    }
  },
  plugins: [
    require('@tailwindcss/forms')
  ]
};