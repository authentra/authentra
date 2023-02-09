import './app.css'
import 'uno.css'
import App from './App.svelte'
import Home from './routes/Home.svelte'
import Test from './routes/Test.svelte';
import { wrap } from 'svelte-spa-router/wrap';

export const routes = {
  "/": Home,
  "/test": Test,
  "/flow/:flow_slug": wrap({
    asyncComponent: () => import('./routes/flow/Flow.svelte')
  })
};

const app = new App({
  target: document.getElementById('app'),
})

export default app
