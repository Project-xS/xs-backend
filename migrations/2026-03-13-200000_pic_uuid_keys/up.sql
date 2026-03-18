ALTER TABLE public.menu_items
    ADD COLUMN pic_key VARCHAR;

ALTER TABLE public.canteens
    ADD COLUMN pic_key VARCHAR;

UPDATE public.menu_items
SET pic_key = gen_random_uuid()::text
WHERE has_pic = true;

UPDATE public.canteens
SET pic_key = gen_random_uuid()::text
WHERE has_pic = true;

ALTER TABLE public.menu_items
    DROP COLUMN has_pic;

ALTER TABLE public.canteens
    DROP COLUMN has_pic;
