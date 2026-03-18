ALTER TABLE public.menu_items
    ADD COLUMN has_pic BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE public.canteens
    ADD COLUMN has_pic BOOLEAN NOT NULL DEFAULT false;

UPDATE public.menu_items
SET has_pic = true
WHERE pic_key IS NOT NULL;

UPDATE public.canteens
SET has_pic = true
WHERE pic_key IS NOT NULL;

ALTER TABLE public.canteens
    DROP COLUMN pic_key;

ALTER TABLE public.menu_items
    DROP COLUMN pic_key;
